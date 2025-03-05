use std::path::PathBuf;

use error::RgError;
use serde::{Deserialize, Serialize};

pub mod backend;
pub mod error;
pub mod stat;
pub mod user_auth;

pub type Result<T> = std::result::Result<T, RgError>;

pub type UserId = u64;
pub type UserPlanId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrafficInfo {
    pub user_id: UserId,
    pub user_plan_id: UserPlanId,
    pub host: String,
    pub local_ip: String,
    pub remote_ip: String,
    pub upload: u64,
    pub download: u64,
}

impl TrafficInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: UserId,
        user_plan_id: UserPlanId,
        host: &str,
        traffic: u64,
        is_upload: bool,
        remote_ip: &str,
        local_ip: &str,
    ) -> TrafficInfo {
        let (upload, download) = if is_upload {
            (traffic, 0)
        } else {
            (0, traffic)
        };
        TrafficInfo {
            user_id,
            user_plan_id,
            upload,
            download,
            host: host.to_string(),
            remote_ip: remote_ip.to_string(),
            local_ip: local_ip.to_string(),
        }
    }

    pub fn get_key(&self) -> String {
        format!("{}-{}-{}", self.host, self.local_ip, self.remote_ip)
    }
}

/// get resources folder path
pub fn get_resource_path() -> PathBuf {
    // get root path
    let mut path = std::env::current_dir().expect("fail to get current dir");
    loop {
        let mut p = path.clone();
        p.push("../../../resources");
        if p.is_dir() {
            return p;
        }
        if !path.pop() {
            panic!("fail to get resource path");
        }
    }
}
