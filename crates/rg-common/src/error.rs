use base64::DecodeError;

#[derive(Debug, thiserror::Error)]
pub enum RgError {
    /// serde error
    #[error("Deserialize error {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Json error {0}")]
    ParseIdError(#[from] std::num::ParseIntError),

    #[error("Connect proxy timeout")]
    ConnectTimeout(#[from] tokio::time::error::Elapsed),

    /// io error
    #[error("io error {0}")]
    IoError(#[from] std::io::Error),

    /// anyhow error
    #[error("anyhow error {0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("invalid auth header")]
    InvalidAuthHeader,

    /// request parse error
    #[error("{0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Invalid request, contains non ascii character")]
    InvalidRequest,

    #[error("Invalid uri {0}")]
    UriParseError(#[from] http::uri::InvalidUri),

    #[error("Uri error {0}")]
    HttpParseError(#[from] http::Error),

    #[error("Request body is empty")]
    EmptyRequest,

    #[error("Socks5 connect unsupported command")]
    UnsupportedCommand,

    #[error("Socks5 connect unsupported address type")]
    UnsupportedAddrType,

    #[error("Socks5 connect request parse error {0}")]
    Socks5ParseError(String),

    #[error("{0}")]
    UsernameParseError(String),

    /// proxy
    #[error("No auth found")]
    NoAuthFound,

    #[error("Auth fail: {0}")]
    AuthFailed(String),

    #[error("Forbidden request")]
    ForbiddenRequest,

    #[error("auth header parse error {0}")]
    DecodeError(#[from] DecodeError),

    #[error("{0}")]
    StringParseError(#[from] std::string::FromUtf8Error),

    #[error("Resolve dns address error {0}")]
    ResolveDnsError(#[from] trust_dns_resolver::error::ResolveError),

    #[error("Send to server error")]
    WebsocketSendError,

    #[error("Connect server error")]
    ConnectServerError,
}
