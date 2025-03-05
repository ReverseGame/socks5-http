pub mod dc_server;

use crate::{conn_set::ConnStat, FilterFn, TrafficFn};
use async_channel::Sender;
use tracing::{error, info};
use rg_acl::auth::dc_auth::IP;
use rg_acl::{AclCenter, AuthCenter};
use rg_common::{error::RgError, user_auth::UserInfo, UserId};
use rg_stat::{RequestType, StatEvent};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{broadcast::Receiver, mpsc::UnboundedSender},
};
use error::{Error, Result};
use http_impl::{IncomingRequest, ProtocolType};

const DEFAULT_USERNAME: &str = "iPOasIsAdmInT0ken";
const DEFAULT_PASSOWRD: &str = "W0rstPassw0rdEveR";

#[async_trait::async_trait]
pub trait ServerBackend {
    async fn handle_connection(&self, conn: TcpStream, remote_addr: SocketAddr) -> Result<()>;

    async fn init_kill_user_connection(&self) -> Sender<UserId>;
}

#[derive(Clone)]
pub struct CommonBackend {
    pub auth: AuthCenter,
    acl: AclCenter,
    conn_set: Arc<ConnStat<tokio::sync::broadcast::Sender<()>>>,
    stat_sender: UnboundedSender<StatEvent>,
}

impl CommonBackend {
    pub fn new(
        auth: AuthCenter,
        acl: AclCenter,
        stat_sender: UnboundedSender<StatEvent>,
    ) -> CommonBackend {
        CommonBackend {
            auth,
            acl,
            stat_sender,
            conn_set: Arc::new(ConnStat::new()),
        }
    }

    pub fn request_stat(&self, r_type: RequestType) {
        if let Err(e) = self.stat_sender.send(StatEvent::Request(r_type)) {
            error!("send request stat error: {}", e);
        }
    }

    pub fn connection_stat(&self, in_conn: i64) {
        if let Err(e) = self.stat_sender.send(StatEvent::Connection(in_conn)) {
            error!("send connection stat error: {}", e);
        }
    }
}

async fn check_is_white(auth_center: &AuthCenter, remote_ip: &str) -> bool {
    info!("remote_ip: {:?}", remote_ip);
    let auth = auth_center.read().await;
    // check auth

    if let Some(user_info) = auth.user_map_get(remote_ip) {
        if user_info.available && user_info.auth_type == IP && remote_ip == user_info.white_ip {
            return true;
        }
    }
    false
}

pub async fn check_user_auth(
    auth_center: &AuthCenter,
    addr: &str,
    remote_ip: &str,
    is_white: bool,
    username: &str,
    password: &str,
) -> error::Result<(bool, UserInfo)> {
    let auth = auth_center.read().await;
    if !auth.in_stock(addr) && DEFAULT_USERNAME.eq(username) && DEFAULT_PASSOWRD.eq(password) {
        return Ok((true, UserInfo::default()));
    }
    // check auth
    let (valid, user_info) = auth.check_auth(&username, &password, addr, remote_ip, is_white);
    // info!("valid: {:?}", valid);
    // if !valid {
    //     req.protocol.respond_auth_result(conn, false, is_white).await?;
    //     return Err(RgError::AuthFailed(format!(
    //         "ip: {}, username: {}, password: {}",
    //         addr, username, password
    //     )));
    // }
    Ok((valid, user_info))
}

async fn http_check_user_auth(
    conn: &mut TcpStream,
    req: &mut IncomingRequest,
    auth_center: &AuthCenter,
    addr: &str,
    remote_ip: &str,
    is_white: bool,
) -> Result<UserInfo> {
    info!("addr: {:?}", addr);
    let (username, password) = if is_white {
        (String::new(), String::new())
    } else {
        match req.protocol.get_user_password() {
            Some((u, p)) => (u, p),
            None => {
                (String::new(), String::new())
                // req.protocol.respond_authorization_required(conn).await?;
                // return Err(RgError::NoAuthFound);
            }
        }
    };
    let auth = auth_center.read().await;
    if !auth.in_stock(addr) && DEFAULT_USERNAME.eq(&username) && DEFAULT_PASSOWRD.eq(&password) {
        return Ok(UserInfo::default());
    }
    // check auth
    let (valid, user_info) = auth.check_auth(&username, &password, addr, remote_ip, is_white);
    info!("valid: {:?}", valid);
    if !valid {
        req.protocol.respond_auth_result(conn, false, is_white).await?;
        return Err(Error::AuthFailed(format!(
            "ip: {}, username: {}, password: {}",
            addr, username, password
        )));
    }
    Ok(user_info)
}

// TODO: may need to spawn two thread to handle upload and download, donot use select
pub async fn io_copy_bidirectional<T>(
    mut src: T,
    mut dst: T,
    upload_filter: Option<FilterFn>,
    download_filter: Option<FilterFn>,
    traffic_handler: TrafficFn,
    mut shutdown_rx: Receiver<()>,
) -> Result<()>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Send,
{
    let mut upload_buf = [0; 50 * 1024];
    let mut download_buf = [0; 50 * 1024];
    loop {
        tokio::select! {
            n = src.read(&mut upload_buf) => {
                let n = n?;
                match n {
                    n if n > 0 => {
                    let size = if let Some(filter) = upload_filter.as_ref() {
                        let b = &upload_buf[..n];
                        let new_b = filter(b);
                        dst.write_all(&new_b).await?;
                        new_b.len()
                    } else {
                        dst.write_all(&upload_buf[..n]).await?;
                        n
                    };
                    traffic_handler(size as u64, true);
                    }
                    0 => {
                        info!("Connection get EOF, release the connection...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        break;
                    }
                    _ => {}
                }
            }
            n = dst.read(&mut download_buf) => {
                let n = n?;
                match n {
                    n if n > 0 => {
                        let size = if let Some(filter) = download_filter.as_ref() {
                            let b = &download_buf[..n];
                            let new_b = filter(b);
                            src.write_all(&new_b).await?;
                            new_b.len()
                        } else {
                            src.write_all(&download_buf[..n]).await?;
                            n
                        };
                    traffic_handler(size as u64, false);
                    }
                    0 => {
                        info!("Connection get EOF, release the connection...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        break;
                    }
                    _ => {}
                }
            }
            _ = shutdown_rx.recv() => {
                info!("get shutdown signal, release the connection...");
                break;
            }
        }
    }
    // close connections
    let _ = src.shutdown().await;
    let _ = dst.shutdown().await;
    Ok(())
}

pub async fn io_copy<S, D>(
    mut src: S,
    mut dst: D,
    filter: Option<FilterFn>,
    traffic_handler: TrafficFn,
    mut shutdown_rx: Receiver<()>,
    upload: bool,
) -> Result<()>
where
    S: AsyncReadExt + Unpin + Send,
    D: AsyncWriteExt + Unpin + Send,
{
    let mut buf = [0; 50 * 1024];
    loop {
        tokio::select! {
            n = src.read(&mut buf) => {
                let n = n?;
                match n {
                    n if n > 0 => {
                    let size = if let Some(filter) = filter.as_ref() {
                        let b = &buf[..n];
                        let new_b = filter(b);
                        dst.write_all(&new_b).await?;
                        new_b.len()
                    } else {
                        dst.write_all(&buf[..n]).await?;
                        n
                    };
                    traffic_handler(size as u64, upload);
                    }
                    0 => {
                        info!("Connection get EOF, release the connection...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        break;
                    }
                    _ => {}
                }
            }
            _ = shutdown_rx.recv() => {
                info!("get shutdown signal, release the connection...");
                break;
            }
        }
    }
    Ok(())
}


fn get_stat_request_type(
    protocol: &ProtocolType,
    method: &http_impl::RequestType,
) -> rg_stat::RequestType {
    match protocol {
        ProtocolType::Http => match method {
            http_impl::RequestType::Connect => rg_stat::RequestType::Https,
            http_impl:: RequestType::Normal => rg_stat::RequestType::Http,
        },
        ProtocolType::Socks5 => RequestType::Socks5,
    }
}