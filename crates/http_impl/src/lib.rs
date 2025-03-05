pub mod proxy;
pub mod https;


use std::str::FromStr;

use ::http::Uri;
use bytes::{BufMut, Bytes, BytesMut};
use tracing::{debug, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use error::{Error, Result};

const HTTP_FORBIDDEN: &[u8] = b"HTTP/1.1 403 Forbidden\r\n\r\n";
const BUFF_SIZE: usize = 4096;
pub struct IncomingRequest {
    pub type_: ProtocolType,
    pub content: Bytes,
    pub protocol: Box<dyn Protocol + Send + Sync>,
}

impl IncomingRequest {
    pub fn hostname(&self) -> String {
        let host = self.protocol.get_host();
        let host = host.host().unwrap_or_default();
        format_hostname(host)
    }
}

#[derive(Debug)]
struct BaseRequestInfo {
    pub host: Uri,
    pub method: RequestType,
    pub auth: Option<(String, String)>,
}

pub async fn parse_incomming_request(
    conn: &mut TcpStream,
    is_white: bool,
) -> Result<IncomingRequest> {
    let request = read_content(conn).await?;
    let bytes = request.clone();
    let (type_, protocol_request) = {
        let mut p = https::HttpRequest::new(bytes.freeze())?;
        if !is_white && p.get_user_password().is_none() {
            p.respond_authorization_required(conn).await?;
            let request = read_content(conn).await?;
            p = https::HttpRequest::new(request.freeze())?;
        }
        (
            ProtocolType::Http,
            Box::new(p) as Box<dyn Protocol + Send + Sync>,
        )
    };
    Ok(IncomingRequest {
        type_,
        content: request.freeze(),
        protocol: protocol_request,
    })
}

#[async_trait::async_trait]
pub trait Protocol {
    fn get_user_password(&self) -> Option<(String, String)>;
    fn get_host(&self) -> Uri;
    fn get_method(&self) -> RequestType;
    async fn respond_auth_result(&mut self, conn: &mut TcpStream, success: bool, is_white: bool) -> Result<()>;
    async fn respond_command_result(&self, conn: &mut TcpStream, success: bool) -> Result<()>;
    async fn respond_authorization_required(&self, conn: &mut TcpStream) -> Result<()>;
    async fn respond_forbidden(&self, conn: &mut TcpStream) -> Result<()> {
        write_all(conn, HTTP_FORBIDDEN).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    Http,
    Socks5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Connect,
    Normal,
}

impl RequestType {
    pub fn default_port(&self) -> u16 {
        match *self {
            RequestType::Connect => 443,
            RequestType::Normal => 80,
        }
    }
}

impl FromStr for RequestType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.to_uppercase();
        Ok(match s.as_str() {
            "CONNECT" => Self::Connect,
            _ => Self::Normal,
        })
    }
}

async fn read_content(conn: &mut TcpStream) -> Result<BytesMut> {
    let mut buf = [0; BUFF_SIZE];
    let mut request = BytesMut::new();
    let timeout = tokio::time::Duration::from_secs(10);
    loop {
        let n = tokio::time::timeout(timeout, conn.read(&mut buf)).await??;
        info!("read {} bytes", n);
        if n == 0 {
            return Err(Error::EmptyRequest);
        }
        request.put_slice(&buf[..n]);
        // if !request.is_ascii() {
        //     return Err(RgError::InvalidRequest);
        // }
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
        if n < BUFF_SIZE {
            break;
        }
    }
    debug!("request: {:?}", String::from_utf8_lossy(&request));
    Ok(request)
}

fn format_hostname(host: &str) -> String {
    let host = host.split(':').next().unwrap_or_default();
    let host = host.rsplit('.').take(3).collect::<Vec<_>>();
    host.into_iter().rev().collect::<Vec<&str>>().join(".")
}

async fn write_all(conn: &mut TcpStream, buf: &[u8]) -> Result<()> {
    conn.write_all(buf).await?;
    conn.flush().await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::format_hostname;

    #[test]
    fn test_parse_hostname() {
        let test_fn = |host, expected: &str| {
            let t = format_hostname(host);
            assert_eq!(t, expected);
        };

        test_fn("www.google.com", "www.google.com");
        test_fn("www.123123.google.com", "123123.google.com");
        test_fn("www.google.com:443", "www.google.com");
        test_fn("a.b.c.google.com:443", "c.google.com");
        test_fn("", "");
    }
}
