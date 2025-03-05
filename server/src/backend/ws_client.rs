use args::ENV_ARG;
use async_channel::Receiver;
use async_trait::async_trait;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tracing::{error, info};
use rg_common::{
    Result, TrafficInfo,
    error::RgError,
    stat::{StatData, StatType},
};
use rg_server_common::message::{ClientMessage, ServerMessage, UserTrafficInfo};
use std::sync::LazyLock;
use std::{
    env,
    sync::{Arc, atomic::Ordering},
};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

pub static SERVER_ADDR: LazyLock<&str> = LazyLock::new(|| match ENV_ARG.as_str() {
    "dev" => "ws://127.0.0.1:8080/ws",
    "beta" => "wss://dcserver.ipoasis.cn/ws",
    "product" => "wss://dcserverws.ipoasis.com/ws",
    _ => "wss://dcserverws.ipoasis.com/ws",
});

use crate::backend::BACKEND_STATUS;

use super::ClientBackend;

type InnerStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[allow(unused)]
#[derive(Debug)]
pub struct WebSocketClient {
    sender: SplitSink<InnerStream, Message>,
    receiver: Arc<Mutex<SplitStream<InnerStream>>>,
}

impl WebSocketClient {
    pub async fn new() -> Result<Self> {
        let server_addr = env::var("RG_SERVER_ADDR").unwrap_or_else(|_| SERVER_ADDR.to_string());

        let (ws_stream, _) = connect_async(server_addr).await.map_err(|e| {
            error!("connect to server error: {}", e);
            RgError::ConnectServerError
        })?;

        let (sender, receiver) = ws_stream.split();
        Ok(Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        })
    }
}

#[async_trait]
impl ClientBackend for WebSocketClient {
    async fn emit_stat(&mut self, stat: StatData) -> Result<()> {
        let data = match stat.stat_type {
            StatType::UserTraffic => {
                let data = serde_json::from_str::<Vec<TrafficInfo>>(&stat.data)?;
                let data = UserTrafficInfo {
                    user_traffics: data,
                    timestamp: stat.timestamp,
                };
                ClientMessage::UserTrafficStat(data)
            }
            _ => ClientMessage::ClientInfoStat(stat),
        };
        self.send(data).await
    }

    async fn ping(&mut self) -> Result<()> {
        let data = Message::Ping(vec![]);
        self.sender
            .send(data)
            .await
            .map_err(|_| RgError::WebsocketSendError)?;
        Ok(())
    }

    async fn send(&mut self, msg: ClientMessage) -> Result<()> {
        self.sender
            .send(Message::Text(serde_json::to_string(&msg)?))
            .await
            .map_err(|_| RgError::WebsocketSendError)?;
        self.sender
            .flush()
            .await
            .map_err(|_| RgError::WebsocketSendError)?;
        Ok(())
    }

    async fn listen(&mut self) -> Receiver<ServerMessage> {
        let (tx, rx) = async_channel::unbounded();
        let receiver = self.receiver.clone();
        tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            loop {
                if let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(msg)) => {
                            if let Ok(msg) = serde_json::from_str::<ServerMessage>(&msg) {
                                if let Err(e) = tx.send(msg).await {
                                    error!("send stat error: {}", e);
                                }
                            } else {
                                error!("parse server message error");
                            }
                        }
                        Ok(Message::Close(_)) => {
                            error!("connection is closed");
                            BACKEND_STATUS.store(false, Ordering::SeqCst);
                            break;
                        }
                        Ok(m) => {
                            info!("receive message: {:?}, ignored", m);
                        }
                        Err(e) => {
                            error!("receive message error: {}", e);
                            BACKEND_STATUS.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                } else {
                    error!("connection is lost");
                    BACKEND_STATUS.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });
        rx
    }
}
