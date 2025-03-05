use async_trait::async_trait;
use http::Uri;
use tokio::net::TcpStream;
use socks5_protocol::{AsyncStreamOperation, AuthMethod, UserKey};
use socks5_protocol::password_method::{Request, Response};
use socks5_protocol::password_method::Status::{Failed, Succeeded};
use crate::socks5_server::AuthExecutor;
use error::{Error, Result};
use crate::backend::check_user_auth;
use crate::backend::dc_server::DC_SERVER_BACKEND;

#[derive(Debug, Default)]
pub struct ServerAuth {
    is_white: bool,
    local_ip: String,
    host: Uri,
    remote_ip: String,
}

impl ServerAuth {
    pub fn new(is_white: bool, local_ip: String, remote_ip: String) -> Self {
        Self { is_white, local_ip, host: Uri::default(), remote_ip }
    }
}

#[async_trait]
impl AuthExecutor for ServerAuth {
    type Output = Result<bool>;

    fn auth_method(&self) -> AuthMethod {
        if self.is_white {
            AuthMethod::NoAuth
        } else {
            AuthMethod::UserPass
        }
    }

    async fn execute(&self, stream: &mut TcpStream) -> Self::Output {
        match self.auth_method() {
            AuthMethod::NoAuth => {
                Ok(true)
            }
            AuthMethod::UserPass => {
                let req = Request::retrieve_from_async_stream(stream).await?;
                let auth = DC_SERVER_BACKEND.auth.clone();
                let (valid, user) = check_user_auth(&auth, &self.host.host().unwrap_or_default(), &self.local_ip, self.is_white, &req.user_key.username, &req.user_key.password).await?;
                let resp = Response::new(if valid { Succeeded } else { Failed });
                resp.write_to_async_stream(stream).await?;
                if valid {
                    Ok(true)
                } else {
                    Err(Error::from(std::io::Error::new(std::io::ErrorKind::Other, "username or password is incorrect")))
                }
            }
            _ => Ok(false)
        }
    }
}