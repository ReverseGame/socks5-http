use base64::engine::general_purpose::STANDARD as BASE64;
use http::Uri;
use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Context};
use base64::Engine;
use bytes::Bytes;
use tracing::error;
use error::{Error, Result};
use tokio::net::TcpStream;
use httparse;
use tokio::io::AsyncWriteExt;
use crate::{Protocol, RequestType};

const HTTP_AUTH_HEADER: &str = "PROXY-AUTHORIZATION";
const SUCCESS: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nUnauthorized\r\n\r\n";
const AUTHENTICATION_REQUIRED: &[u8] = b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"Proxy-Login\"\r\n\r\n";

#[derive(Debug)]
pub struct HttpRequest {
    inner: BaseRequestInfo,
}

#[derive(Debug)]
struct BaseRequestInfo {
    pub host: Uri,
    pub method: RequestType,
    pub auth: Option<(String, String)>,
}



impl HttpRequest {
    pub fn new(buf: Bytes) -> Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);
        req.parse(&buf)
            .map_err(|e| Error::from( format!("parse content error {:?}", e)))?;
        let method = req
            .method
            .context("do not find method")?
            .to_string()
            .to_uppercase();
        let path = req.path.context("do not find path").map_err(|e| Error::from(e))?;
        let uri = Uri::from_str(path)?;
        let mut header_map = HashMap::new();

        for header in headers.into_iter() {
            if header.name.is_empty() {
                continue;
            }
            header_map.insert(
                header.name.to_string().to_uppercase(),
                String::from_utf8(header.value.to_vec())?,
            );
        }
        let base = BaseRequestInfo {
            method: RequestType::from_str(&method)?,
            host: uri,
            auth: get_auth_header(&header_map),
        };
        Ok(Self { inner: base })
    }
}

#[async_trait::async_trait]
impl Protocol for HttpRequest {
    fn get_user_password(&self) -> Option<(String, String)> {
        self.inner.auth.clone()
    }

    fn get_host(&self) -> Uri {
        self.inner.host.clone()
    }

    fn get_method(&self) -> RequestType {
        self.inner.method
    }

    async fn respond_auth_result(&mut self, conn: &mut TcpStream, success: bool, _is_white: bool) -> Result<()> {
        if !success {
            write_all(conn, UNAUTHORIZED).await?;
        }
        Ok(())
    }

    async fn respond_command_result(&self, conn: &mut TcpStream, success: bool) -> Result<()> {
        if success {
            write_all(conn, SUCCESS).await?;
        }
        Ok(())
    }

    async fn respond_authorization_required(&self, conn: &mut TcpStream) -> Result<()> {
        write_all(conn, AUTHENTICATION_REQUIRED).await?;
        Ok(())
    }
}

async fn write_all(conn: &mut TcpStream, buf: &[u8]) -> Result<()> {
    conn.write_all(buf).await?;
    conn.flush().await?;
    Ok(())
}

fn get_auth_header(headers: &HashMap<String, String>) -> Option<(String, String)> {
    let basic_auth = headers.get(HTTP_AUTH_HEADER)?;
    let split = basic_auth.split(' ').collect::<Vec<_>>();
    if split.len() != 2 {
        error!("invalid auth header: {}", basic_auth);
        return None;
    }
    decode_basic_auth(split[1]).ok()
}

fn decode_basic_auth(auth: &str) -> Result<(String, String)> {
    let auth = auth.trim();
    let auth = Engine::decode(&BASE64, auth)?;
    let auth = String::from_utf8(auth)?;
    let auth = auth.split(':').collect::<Vec<&str>>();
    if auth.len() != 2 {
        return Err(Error::InvalidAuthHeader);
    }
    Ok((auth[0].to_string(), auth[1].to_string()))
}

// #[cfg(test)]
// mod tests {
//
//     #[test]
//     fn test_parse_connect_domain_port_request() {
//         let request = b"CONNECT www.baidu.com:443 HTTP/1.1\r\nHost: www.baidu.com:443\r\nProxy-Connection: keep-alive\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)\r\n\r\n";
//         let request = crate::https::HttpRequest::new(request.to_vec().into());
//         assert!(request.is_ok());
//         let request = request.unwrap();
//         assert!(request.get_user_password().is_none());
//         assert_eq!(request.get_host().host().unwrap(), "www.baidu.com");
//         assert_eq!(request.get_method(), crate::RequestType::Connect);
//
//         let request = b"CONNECT www.baidu.com:443 HTTP/1.1\r\nHost: www.baidu.com:443\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ=\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)\r\n\r\n";
//         let request = crate::https::HttpRequest::new(request.to_vec().into());
//         assert!(request.is_ok());
//         let request = request.unwrap();
//         assert!(request.inner.auth.is_some());
//         let auth = request.inner.auth.unwrap();
//         assert_eq!(&auth.0, "username");
//         assert_eq!(&auth.1, "password");
//     }
//
//     #[test]
//     fn test_parse_connect_ip_port() {
//         let request = b"CONNECT 127.0.0.1 HTTP/1.1\r\nHost: 127.0.0.1\r\nProxy-Connection: keep-alive\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)\r\n\r\n";
//         let request = crate::https::HttpRequest::new(request.to_vec().into());
//         assert!(request.is_ok());
//         let request = request.unwrap();
//         assert!(request.get_user_password().is_none());
//         assert_eq!(request.get_host().host().unwrap(), "127.0.0.1");
//         assert_eq!(request.get_method(), crate::RequestType::Connect);
//         println!("{:?}", request.inner.host.authority());
//     }
// }
