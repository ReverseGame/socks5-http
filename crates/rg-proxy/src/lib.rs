pub mod backend;
mod conn_set;
pub mod proxy_server;
mod resolver;
mod util;
pub mod socks5_server;

use bytes::Bytes;
use rg_common::{user_auth::UserInfo, Result, TrafficInfo};
use rg_stat::StatEvent;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};

type FilterFn = Box<dyn Fn(&[u8]) -> Bytes + Send + 'static>;
type TrafficFn = Box<dyn Fn(u64, bool) + Send + 'static>;

#[async_trait::async_trait]
pub trait Server {
    async fn start(&self) -> Result<()>;

    async fn stop(&self) -> Result<()>;

    async fn handle_connection(&self, conn: TcpStream, remote_addr: SocketAddr) {
        let _ = conn.set_linger(Some(Duration::from_secs(0)));

        self._handle(conn, remote_addr).await;
    }

    async fn _handle(&self, conn: TcpStream, remote_addr: SocketAddr);
}

fn get_traffic_fn(
    sender: UnboundedSender<StatEvent>,
    user_info: &UserInfo,
    hostname: String,
    local_ip: String,
    remote_ip: String,
) -> TrafficFn {
    let user_id = user_info.user_id;
    let user_plan_id = user_info.user_plan_id;
    Box::new(move |traffic, is_upload| {
        let msg = TrafficInfo::new(
            user_id,
            user_plan_id,
            &hostname,
            traffic,
            is_upload,
            &remote_ip,
            &local_ip,
        );
        let _ = sender.send(StatEvent::Traffic(msg));
    })
}
