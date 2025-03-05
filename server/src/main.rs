mod utils;
mod server_client;
mod emit_client;
mod backend;

use {
    socks5_protocol::{Address, Reply, UdpHeader},
    rg_proxy::socks5_server::{auth, connection::associate, AssociatedUdpSocket, ClientConnection, IncomingConnection, UdpAssociate},
    error::{Error, Result},
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{atomic::AtomicBool, Arc},
};
use std::process::exit;
use tokio::{
    io,
    net::{TcpStream, UdpSocket},
    sync::Mutex,
};
use rg_acl::{acl::DefaultAclRule, auth::dc_auth::DcAuthenticator};
use as_any::AsAny;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use rg_common::stat::StatType;
use crate::utils::get_local_ip_port;
use tracing::{info, error};
use rg_proxy::backend::{CommonBackend, ServerBackend};
use rg_proxy::backend::dc_server::{init, DcServerBackend, DC_SERVER_BACKEND};
use strum::IntoEnumIterator;
use rg_proxy::proxy_server::ProxyServer;
use rg_proxy::Server;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // create client first
    let mut client =server_client::ServerClient::new().await;

    let acl_center = Arc::new(RwLock::new(DefaultAclRule {}));
    let auth_center = Arc::new(RwLock::new(DcAuthenticator::default()));

    // create statistic manager
    let (stat_sender, stat_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut stat_manager = rg_stat::StatisticManager::new(stat_receiver);

    for stat_type in StatType::iter() {
        let t = stat_manager.subscribe(stat_type);
        client.add_subscribe(t).await;
    }
    init(stat_sender).await;
    // create proxy server
    // let dc_backend = DcServerBackend::new(CommonBackend::new(
    //     auth_center.clone(),
    //     acl_center.clone(),
    //     stat_sender,
    // ));

    let kill_user_sender = DC_SERVER_BACKEND.init_kill_user_connection().await;
    // start listening
    // let backend = Arc::new(dc_backend);
    let local_ip_ports = get_local_ip_port().await;
    let mut servers = Vec::new();
    for ip in local_ip_ports {
        let listener = match tokio::net::TcpListener::bind(ip).await {
            Ok(listener) => listener,
            Err(e) => {
                error!("fail to bind {}, error: {}", ip, e);
                continue;
            }
        };
        servers.push(ProxyServer::new(listener, DC_SERVER_BACKEND.clone()).await);
    }

    info!("start stat manager");
    // start run
    tokio::spawn(async move {
        stat_manager.run().await;
    });

    info!("start run client");
    tokio::spawn(async move {
        client.run(auth_center, acl_center, kill_user_sender).await;
    });

    info!("start proxy server");
    let mut handlers = Vec::new();
    for server in servers {
        let handler = tokio::spawn(async move {
            server.start().await.expect("start server failed");
        });
        handlers.push(handler);
    }
    futures::future::join_all(handlers).await;
    exit(0);
}
