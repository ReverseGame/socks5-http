use crate::socks5_server::auth::UserKeyAuth;
use crate::socks5_server::handle_conn::handle;
use crate::socks5_server::server_auth::ServerAuth;
use crate::socks5_server::{handle_conn, AuthAdaptor, IncomingConnection};
use crate::{backend::io_copy, get_traffic_fn, resolver::resolve_host, util::remove_headers, FilterFn};
use async_channel::Sender;
use error::{Error, Result};
use http_impl::parse_incomming_request;
use rg_acl::acl::DefaultAclRule;
use rg_acl::auth::dc_auth::DcAuthenticator;
use rg_common::UserId;
use rg_stat::StatEvent;
use socks5_http::{Sock5Http, Sock5OrHttp};
use std::sync::{Arc, LazyLock};
use std::{
    net::{IpAddr, SocketAddr},
    ops::{Deref, DerefMut},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{OnceCell, RwLock};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
};
use tracing::{debug, error, info};

use super::{check_is_white, get_stat_request_type, http_check_user_auth, CommonBackend, ServerBackend};

const RETRY_LIMIT: u32 = 3;

pub static DC_SERVER_BACKEND_ONCE: OnceCell<Arc<DcServerBackend>> = OnceCell::const_new();
pub static DC_SERVER_BACKEND: LazyLock<Arc<DcServerBackend>> =
    LazyLock::new(|| DC_SERVER_BACKEND_ONCE.get().expect("DcServerBackend not initialized").clone());

pub async fn init(stat_sender: UnboundedSender<StatEvent>) {
    DC_SERVER_BACKEND_ONCE
        .get_or_init(|| async move {
            let acl_center = Arc::new(RwLock::new(DefaultAclRule {}));
            let auth_center = Arc::new(RwLock::new(DcAuthenticator::default()));

            let dc_backend = DcServerBackend::new(CommonBackend::new(auth_center.clone(), acl_center.clone(), stat_sender));
            Arc::new(dc_backend)
        })
        .await;
}

pub struct DcServerBackend {
    pub inner: CommonBackend,
}

impl DcServerBackend {
    pub fn new(inner: CommonBackend) -> Self {
        DcServerBackend { inner }
    }
}

impl Deref for DcServerBackend {
    type Target = CommonBackend;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DcServerBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[async_trait::async_trait]
impl ServerBackend for DcServerBackend {
    async fn handle_connection(&self, conn: TcpStream, remote_addr: SocketAddr) -> Result<()> {
        let remote_ip = remote_addr.ip().to_string();
        info!("remote_ip: {:?}", remote_ip);
        let is_white = check_is_white(&self.auth, &remote_ip).await;
        info!("is white: {}", is_white);
        let local_ip_addr = conn.local_addr()?.ip();
        let local_ip = local_ip_addr.to_string();

        let mut conn = Sock5Http::new(conn);
        match conn.socks5_or_http().await.unwrap() {
            Sock5OrHttp::Sock5 => {
                let auth = ServerAuth::new(true, local_ip, remote_ip);
                let auth = Arc::new(auth);
                if let Err(e) = handle(auth, conn.stream.into_inner()).await {
                    tracing::error!("handle connection error: {}", e);
                }
            }
            Sock5OrHttp::Http => {
                let mut conn = conn.stream.into_inner();
                let mut req = parse_incomming_request(&mut conn, is_white).await?;
                let user_info = http_check_user_auth(&mut conn, &mut req, &self.auth, &local_ip, &remote_ip, is_white).await?;
                let method = req.protocol.get_method();
                let target_host = req.protocol.get_host();
                let host = target_host.host().unwrap_or_default();
                info!("method: {:?}, host: {}", method, host);
                self.request_stat(get_stat_request_type(&req.type_, &method));

                // check acl
                if !self.acl.read().await.check(&user_info, host, &local_ip) {
                    req.protocol.respond_forbidden(&mut conn).await?;
                    error!("forbidden request from user: {:?}, host: {}", user_info, host);
                    return Err(Error::ForbiddenRequest);
                }

                // resolve dns hostname
                let port = if let Some(p) = target_host.port() {
                    p.as_u16()
                } else {
                    method.default_port()
                };
                let target_addr = resolve_host(host, port).await?;
                let mut size = 0;
                // connect to target website
                let mut out_conn = connect_target(target_addr, local_ip_addr).await?;
                let _ = out_conn.set_linger(Some(Duration::from_secs(0)));

                let traffic_fn = get_traffic_fn(
                    self.stat_sender.clone(),
                    &user_info,
                    req.hostname(),
                    local_ip.clone(),
                    remote_addr.ip().to_string(),
                );
                let mut header_filter = None;

                // handle http request
                let content = &req.content;
                let new_content = remove_headers(content, "PROXY");
                out_conn.write_all(&new_content).await?;
                size += new_content.len();
                header_filter = Some(Box::new(|content: &[u8]| remove_headers(content, "PROXY")) as FilterFn);

                traffic_fn(size as u64, true);
                let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(10);
                let id = &shutdown_tx as *const _ as usize;
                self.conn_set.add(user_info.user_id, id, shutdown_tx.clone());

                let (mut src_read, mut src_write) = conn.split();
                let (mut dst_read, mut dst_write) = out_conn.split();

                let c_traffic_fn = get_traffic_fn(
                    self.stat_sender.clone(),
                    &user_info,
                    req.hostname(),
                    local_ip.clone(),
                    remote_addr.ip().to_string(),
                );
                let c_shutdown = shutdown_tx.subscribe();
                let t1 = io_copy(&mut src_read, &mut dst_write, header_filter, c_traffic_fn, shutdown_rx, true);
                let t2 = io_copy(&mut dst_read, &mut src_write, None, traffic_fn, c_shutdown, false);
                let res = tokio::select! {
                    res = t1 => {
                        info!("io copy from src to dst finished");
                        res
                    }
                    res = t2 => {
                        info!("io copy from dst to src finished");
                        res
                    }
                };
                // let res = tokio::join!(t1, t2);
                debug!("io copy finish: {:?}", res);
                let _ = shutdown_tx.send(());
                info!("remove shutdown tx from kill list...");
                self.conn_set.remove(user_info.user_id, id);
            }
        }

        // if !self.acl.read().await.check(&user_info, host, &local_ip) {
        //     req.protocol.respond_forbidden(&mut conn).await?;
        //     error!(
        //         "forbidden request from user: {:?}, host: {}",
        //         user_info, host
        //     );
        //     return Err(RgError::ForbiddenRequest);
        // }
        //
        // info!("is white: {}", is_white);
        // let server_auth = Arc::new(ServerAuth::new(true));
        // if let Err(e) = handle(server_auth, conn).await {
        //     tracing::error!("handle connection error: {}", e);
        // }
        // let mut req = parse_incomming_request(&mut conn, is_white).await?;
        // info!("new request from: {:?}", remote_addr);
        // // get requested local ip address
        // let local_ip_addr = conn.local_addr()?.ip();
        // let local_ip = local_ip_addr.to_string();
        // info!("local ip: {}", local_ip);
        // let user_info = check_user_auth(
        //     &mut conn, &mut req, &self.auth, &local_ip, &remote_ip, is_white,
        // )
        // .await?;
        //
        // let method = req.protocol.get_method();
        // let target_host = req.protocol.get_host();
        // let host = target_host.host().unwrap_or_default();
        // info!("method: {:?}, host: {}", method, host);
        // self.request_stat(get_stat_request_type(&req.type_, &method));
        //
        // // check acl
        // if !self.acl.read().await.check(&user_info, host, &local_ip) {
        //     req.protocol.respond_forbidden(&mut conn).await?;
        //     error!(
        //         "forbidden request from user: {:?}, host: {}",
        //         user_info, host
        //     );
        //     return Err(RgError::ForbiddenRequest);
        // }
        //
        // // resolve dns hostname
        // let port = if let Some(p) = target_host.port() {
        //     p.as_u16()
        // } else {
        //     method.default_port()
        // };
        // let target_addr = resolve_host(host, port).await?;
        // let mut size = 0;
        // // connect to target website
        // let mut out_conn = connect_target(target_addr, local_ip_addr).await?;
        // let _ = out_conn.set_linger(Some(Duration::from_secs(0)));
        //
        // let traffic_fn = get_traffic_fn(
        //     self.stat_sender.clone(),
        //     &user_info,
        //     req.hostname(),
        //     local_ip.clone(),
        //     remote_addr.ip().to_string(),
        // );
        // let mut header_filter = None;
        //
        // if method == RequestType::Connect || req.type_ == ProtocolType::Socks5 {
        //     req.protocol.respond_command_result(&mut conn, true).await?;
        // } else {
        //     // handle http request
        //     let content = &req.content;
        //     let new_content = remove_headers(content, "PROXY");
        //     out_conn.write_all(&new_content).await?;
        //     size += new_content.len();
        //     header_filter =
        //         Some(Box::new(|content: &[u8]| remove_headers(content, "PROXY")) as FilterFn);
        // }
        // traffic_fn(size as u64, true);
        // let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(10);
        // let id = &shutdown_tx as *const _ as usize;
        // self.conn_set
        //     .add(user_info.user_id, id, shutdown_tx.clone());
        //
        // let (mut src_read, mut src_write) = conn.split();
        // let (mut dst_read, mut dst_write) = out_conn.split();
        //
        // let c_traffic_fn = get_traffic_fn(
        //     self.stat_sender.clone(),
        //     &user_info,
        //     req.hostname(),
        //     local_ip.clone(),
        //     remote_addr.ip().to_string(),
        // );
        // let c_shutdown = shutdown_tx.subscribe();
        // let t1 = io_copy(
        //     &mut src_read,
        //     &mut dst_write,
        //     header_filter,
        //     c_traffic_fn,
        //     shutdown_rx,
        //     true,
        // );
        // let t2 = io_copy(
        //     &mut dst_read,
        //     &mut src_write,
        //     None,
        //     traffic_fn,
        //     c_shutdown,
        //     false,
        // );
        // let res = tokio::select! {
        //     res = t1 => {
        //         info!("io copy from src to dst finished");
        //         res
        //     }
        //     res = t2 => {
        //         info!("io copy from dst to src finished");
        //         res
        //     }
        // };
        // // let res = tokio::join!(t1, t2);
        // debug!("io copy finish: {:?}", res);
        // let _ = shutdown_tx.send(());
        // info!("remove shutdown tx from kill list...");
        // self.conn_set.remove(user_info.user_id, id);
        Ok(())
    }

    async fn init_kill_user_connection(&self) -> Sender<UserId> {
        let (tx, rx) = async_channel::unbounded();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(user_id) = rx.recv().await {
                    inner.conn_set.kill_user(user_id);
                }
            }
        });
        tx
    }
}
async fn connect_target(addr: SocketAddr, local_ip: IpAddr) -> Result<TcpStream> {
    let mut retry = 0;
    loop {
        match _connect_target(addr, local_ip).await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                error!("error connecting to target: {}", e);
                retry += 1;
                if retry >= RETRY_LIMIT {
                    return Err(e);
                }
            }
        }
    }
}

async fn _connect_target(addr: SocketAddr, local_ip: IpAddr) -> Result<TcpStream> {
    let socket = TcpSocket::new_v4()?;
    let mut sock_addr = socket.local_addr()?;
    sock_addr.set_ip(local_ip);
    sock_addr.set_port(0);
    socket.set_keepalive(true)?;
    // socket.set_reuseport(true)?;
    socket.bind(sock_addr)?;
    let conn = socket.connect(addr).await?;
    Ok(conn)
}
