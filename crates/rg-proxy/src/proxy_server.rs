use std::{net::SocketAddr, ops::Deref, sync::Arc};

use tracing::{debug, error};
use rg_common::Result;
use tokio::net::{TcpListener, TcpStream};

use crate::{
    backend::{CommonBackend, ServerBackend},
    Server,
};

pub struct ProxyServer<T>
where
    T: ServerBackend + Deref<Target = CommonBackend> + Send + Sync,
{
    listener: TcpListener,
    inner: Arc<T>,
}

#[async_trait::async_trait]
impl<T> Server for ProxyServer<T>
where
    T: ServerBackend + Deref<Target = CommonBackend> + Send + Sync + 'static,
{
    async fn start(&self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((conn, remote_addr)) => {
                    debug!("accept connection from: {}", remote_addr);
                    self.inner.request_stat(rg_stat::RequestType::None);
                    self.inner.connection_stat(1);
                    self.handle_connection(conn, remote_addr).await;
                    self.inner.connection_stat(-1);
                }
                Err(e) => {
                    error!("accept connection failed: {}", e);
                }
            }
        }
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn _handle(&self, conn: TcpStream, remote_addr: SocketAddr) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            if let Err(e) = inner.handle_connection(conn, remote_addr).await {
                error!("handle connection error: {}", e);
            }
        });
    }
}

impl<T> ProxyServer<T>
where
    T: ServerBackend + Deref<Target = CommonBackend> + Send + Sync,
{
    pub async fn new(listener: TcpListener, inner: Arc<T>) -> Self {
        ProxyServer { listener, inner }
    }
}
