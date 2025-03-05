use async_channel::{Receiver, Sender};
use tracing::{debug, error, info};
use rg_acl::{AclCenter, AuthCenter};
use rg_common::{Result, UserId, stat::StatData};
use rg_server_common::message::ServerMessage;
use std::{
    env,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::Mutex;

use crate::{
    backend::{BACKEND_STATUS, ClientBackend, std_client::StdClient, ws_client::WebSocketClient},
    emit_client::EmitClient,
};

pub struct ServerClient {
    // emit client, send stat to channel
    emit_client: Arc<Mutex<EmitClient>>,
    // receive stat from channel and send to backend
    stat_reciever: async_channel::Receiver<StatData>,
    // client backend, communication with server
    backend: Box<dyn ClientBackend + Send + Sync>,
}

impl ServerClient {
    pub async fn new() -> Self {
        let (sender, reciever) = async_channel::unbounded();
        let emit_client = EmitClient::new(sender);

        let backend = match Self::init_backend().await {
            Ok(backend) => backend,
            Err(e) => {
                panic!("init backend error: {}", e);
            }
        };

        Self {
            emit_client: Arc::new(Mutex::new(emit_client)),
            stat_reciever: reciever,
            backend,
        }
    }

    async fn init_backend() -> Result<Box<dyn ClientBackend + Send + Sync>> {
        let be = env::var("RG_BACKEND").unwrap_or_else(|_| "server".to_string());
        let mut backend: Box<dyn ClientBackend + Send + Sync> =
            match be.eq_ignore_ascii_case("server") {
                true => Box::new(WebSocketClient::new().await?),
                false => Box::new(StdClient::new()),
            };
        info!("send authenticate message");
        backend.authenticate().await?;
        info!("send local ip range");
        backend.upload_local_ips().await?;
        BACKEND_STATUS.store(true, Ordering::SeqCst);
        Ok(backend)
    }

    pub async fn add_subscribe(&mut self, subscribe: tokio::sync::broadcast::Receiver<StatData>) {
        self.emit_client.lock().await.add_subscribe(subscribe);
    }

    /// start the client
    pub async fn run(
        &mut self,
        auth_center: AuthCenter,
        acl_center: AclCenter,
        kill_user_sender: Sender<UserId>,
    ) {
        let emit_client = self.emit_client.clone();
        tokio::spawn(async move {
            emit_client.lock().await.run().await;
        });

        loop {
            let status = BACKEND_STATUS.load(Ordering::SeqCst);
            if !status {
                // server connection is down
                // try re-init backend
                debug!("try to reconnect backend");
                match Self::init_backend().await {
                    Ok(backend) => {
                        self.backend = backend;
                    }
                    Err(e) => {
                        error!("reconnect backend error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }

            // handle server message
            let recv = self.backend.listen().await;
            tokio::spawn(handle_server_message(
                recv,
                auth_center.clone(),
                acl_center.clone(),
                kill_user_sender.clone(),
            ));

            // handle stat
            let mut timer = tokio::time::interval(Duration::from_secs(5));
            timer.reset();
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        if let Err(e) = self.backend.ping().await {
                            error!("ping server error, connection may be lost: {}", e);
                        }
                    }
                    stat = self.stat_reciever.recv() => {
                        if let Ok(stat) = stat {
                            if let Err(e) = self.backend.emit_stat(stat).await {
                                error!("emit stat error: {}", e);
                            }
                        }
                    }
                }
                let status = BACKEND_STATUS.load(Ordering::SeqCst);
                if !status {
                    break;
                }
            }
        }
    }
}

async fn handle_server_message(
    channel: Receiver<ServerMessage>,
    auth_center: AuthCenter,
    acl_center: AclCenter,
    kill_user_sender: Sender<UserId>,
) {
    loop {
        if let Ok(msg) = channel.recv().await {
            debug!("receive server message: {:?}", msg);
            match msg {
                ServerMessage::DisableUser(id) => {
                    // kill user
                    if let Err(e) = kill_user_sender.send(id).await {
                        error!("send kill user error: {}", e);
                    }
                }
                ServerMessage::AclData(data) => {
                    // update acl
                    {
                        let mut acl = acl_center.write().await;
                        acl.update(&data);
                    }
                }
                ServerMessage::UserAuth(user_infos) => {
                    // update auth
                    {
                        let mut auth = auth_center.write().await;
                        auth.update_all(user_infos);
                    }
                }
                ServerMessage::UpdateUser(user) => {
                    // update stat
                    // stat_sender.send(stat).await;
                    {
                        let auth = auth_center.write().await;
                        auth.update_user_info(user);
                    }
                }
                ServerMessage::UserWhiteList(data) => {
                    let mut auth = auth_center.write().await;
                    auth.update_white_list(data);
                }
            }
        }
        if !BACKEND_STATUS.load(Ordering::SeqCst) {
            break;
        }
    }
}
