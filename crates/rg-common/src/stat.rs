use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, EnumIter, Serialize, Deserialize)]
pub enum StatType {
    UserTraffic,
    TrafficTotal,
    Request,
    Connection,
    System,
}

impl Display for StatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StatType::UserTraffic => "user_traffic",
                StatType::TrafficTotal => "traffic_total",
                StatType::Request => "request",
                StatType::Connection => "connection",
                StatType::System => "system",
            }
        )
    }
}

impl From<&str> for StatType {
    fn from(s: &str) -> Self {
        match s {
            "user_traffic" => StatType::UserTraffic,
            "traffic_total" => StatType::TrafficTotal,
            "request" => StatType::Request,
            "connection" => StatType::Connection,
            "system" => StatType::System,
            _ => panic!("unknown stat type"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatData {
    pub stat_type: StatType,
    pub data: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatSnapshot {
    pub alive_in_connection: i64,
    // pub alive_out_connection: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStatSnapshot {
    pub total_request: u64,
    pub http_request: u64,
    pub https_request: u64,
    pub socks5_request: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficTotalStatSnapshot {
    pub total: u64,
    pub upload: u64,
    pub download: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatSnapshot {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub network_receive: u64,
    pub network_transmit: u64,
    pub ping_latency: Option<f32>,
}
