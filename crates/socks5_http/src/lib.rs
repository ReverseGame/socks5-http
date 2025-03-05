use error::Result;
use hyper_util::rt::TokioIo;
use socks5_protocol::Version;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub struct Sock5Http {
    pub stream: TokioIo<TcpStream>,
}

pub enum Sock5OrHttp {
    Sock5,
    Http,
}

impl Sock5Http {
    pub fn new(sock5_or_http: TcpStream) -> Self {
        Self {
            stream: TokioIo::new(sock5_or_http),
        }
    }

    pub async fn socks5_or_http(&mut self) -> Result<Sock5OrHttp> {
        let mut ver = [0u8; 1];
        self.stream.inner_mut().read_exact(&mut ver).await?;
        let version = Version::try_from(ver[0])?;
        if version == Version::V5 {
            Ok(Sock5OrHttp::Sock5)
        } else {
            Ok(Sock5OrHttp::Http)
        }
    }
}
