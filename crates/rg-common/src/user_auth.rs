use serde::{Deserialize, Serialize};

use crate::{UserId, UserPlanId};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct UserInfo {
    pub user_id: UserId,
    pub user_plan_id: UserPlanId,
    pub username: String,
    pub password: String,
    pub white_ip: String,
    pub auth_type: String,
    // ips allowed to access
    pub ips: Vec<String>,
    pub available: bool,
}

impl UserInfo {
    pub fn new(
        user_id: UserId,
        user_plan_id: UserPlanId,
        username: &str,
        password: &str,
        white_ip: &str,
        auth_type: &str,
        ips: Vec<String>,
    ) -> Self {
        Self {
            user_id,
            user_plan_id,
            username: username.to_string(),
            password: password.to_string(),
            white_ip: white_ip.to_string(),
            auth_type: auth_type.to_string(),
            ips,
            available: true,
        }
    }

    pub fn clone_id(user_info: &UserInfo) -> Self {
        Self {
            user_id: user_info.user_id,
            user_plan_id: user_info.user_plan_id,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhiteListData {
    pub ip: String,
    pub username: String,
    pub password: String,
    pub user_id: UserId,
}

#[allow(dead_code)]
impl WhiteListData {
    pub fn new_with_id(ip: &str, username: &str, password: &str, user_id: UserId) -> Self {
        Self {
            ip: ip.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            user_id,
        }
    }

    pub fn new_default_id(ip: &str, username: &str, password: &str) -> Self {
        Self {
            ip: ip.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            ..Default::default()
        }
    }
}
