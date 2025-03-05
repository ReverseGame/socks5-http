use std::net::SocketAddr;

use anyhow::Context;
use error::Result;

use trust_dns_resolver::{
    name_server::{GenericConnector, TokioRuntimeProvider},
    AsyncResolver,
};

lazy_static::lazy_static! {
    static ref RESOLVER: AsyncResolver<GenericConnector<TokioRuntimeProvider>> = AsyncResolver::tokio_from_system_conf().expect("unable to create resolver from system conf");
}

pub async fn resolve_host(host: &str, port: u16) -> Result<SocketAddr> {
    let response = RESOLVER.lookup_ip(host).await?;
    let ip = response.iter().next().context("no addresses returned")?;
    Ok(SocketAddr::new(ip, port))
}
