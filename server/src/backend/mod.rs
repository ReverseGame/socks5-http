pub mod std_client;
pub mod ws_client;

use std::sync::atomic::AtomicBool;

use async_channel::Receiver;
use async_trait::async_trait;
use rg_common::{Result, stat::StatData};
use rg_server_common::message::{ClientMessage, ServerIpInfo, ServerMessage};

use crate::utils::get_config;

/// backend status, if backend is running or not
/// used for reconnect if backend is down
pub static BACKEND_STATUS: AtomicBool = AtomicBool::new(false);

#[async_trait]
pub trait ClientBackend {
    async fn emit_stat(&mut self, stat: StatData) -> Result<()>;
    async fn ping(&mut self) -> Result<()>;
    async fn send(&mut self, msg: ClientMessage) -> Result<()>;
    async fn listen(&mut self) -> Receiver<ServerMessage>;

    async fn authenticate(&mut self) -> Result<()> {
        let msg = ClientMessage::Authenticate(rg_server_common::auth::AUTH_PRIVATE_KEY.to_string());
        self.send(msg).await?;
        Ok(())
    }

    async fn upload_local_ips(&mut self) -> Result<()> {
        let config = get_config().await;
        let ip_range = config.ip_range.iter().map(|x| x.to_string()).collect();
        let data = ServerIpInfo {
            ip_range,
            local_ip: config.local_ip.clone(),
            port_start: config.port_start,
            port_end: Some(config.port_end),
            offset: Some(config.offset),
            extra_ips: config.extra_ips.clone(),
            ..Default::default()
        };
        let msg = ClientMessage::IpRange(data);
        self.send(msg).await?;
        Ok(())
    }
}
