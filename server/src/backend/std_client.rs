use async_channel::Receiver;
use async_trait::async_trait;
use tracing::error;
use rg_common::{Result, stat::StatData};
use rg_server_common::message::{ClientMessage, ServerMessage};

use super::ClientBackend;

pub struct StdClient;

impl StdClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ClientBackend for StdClient {
    async fn emit_stat(&mut self, stat: StatData) -> Result<()> {
        println!("receive stat: {:?}", stat);
        Ok(())
    }

    async fn send(&mut self, msg: ClientMessage) -> Result<()> {
        println!("send msg: {:?}", msg);
        Ok(())
    }

    async fn ping(&mut self) -> Result<()> {
        println!("client pinged");
        Ok(())
    }

    async fn listen(&mut self) -> Receiver<ServerMessage> {
        let (tx, rx) = async_channel::unbounded();
        tokio::spawn(async move {
            loop {
                // get input from stdin
                let mut input = String::new();
                if let Err(e) = std::io::stdin().read_line(&mut input) {
                    error!("read stdin error: {}", e);
                    continue;
                }
                match serde_json::from_str::<ServerMessage>(&input) {
                    Ok(msg) => {
                        if let Err(e) = tx.send(msg).await {
                            error!("send stat error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("parse server message error: {}", e);
                    }
                }
            }
        });
        rx
    }
}
