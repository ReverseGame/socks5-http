use std::sync::atomic::AtomicU64;

use rg_common::stat::RequestStatSnapshot;

use crate::StatCollectable;

#[derive(Debug, Clone)]
pub enum RequestType {
    Http,
    Https,
    Socks5,
    // for total request
    None,
}

pub struct RequestStat {
    pub total_request: AtomicU64,
    pub http_request: AtomicU64,
    pub https_request: AtomicU64,
    pub socks5_request: AtomicU64,
}

impl StatCollectable for RequestStat {
    fn stat_type(&self) -> crate::StatType {
        crate::StatType::Request
    }

    fn _collect(&mut self) -> String {
        let snap = RequestStatSnapshot {
            total_request: self
                .total_request
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            http_request: self
                .http_request
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            https_request: self
                .https_request
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            socks5_request: self
                .socks5_request
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
        };
        serde_json::to_string(&snap).unwrap_or_default()
    }
}

impl RequestStat {
    pub fn new() -> Self {
        Self {
            total_request: AtomicU64::new(0),
            http_request: AtomicU64::new(0),
            https_request: AtomicU64::new(0),
            socks5_request: AtomicU64::new(0),
        }
    }

    pub fn add(&self, type_: RequestType) {
        match type_ {
            RequestType::None => &self.total_request,
            RequestType::Http => &self.http_request,
            RequestType::Https => &self.https_request,
            RequestType::Socks5 => &self.socks5_request,
        }
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
