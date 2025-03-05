pub mod dc_auth;

use tracing::error;
use rg_common::{
    user_auth::{UserInfo, WhiteListData},
    UserId,
};

pub trait Authenticator {
    /// check if user is authenticated by username and password
    fn check_auth(
        &self,
        username: &str,
        password: &str,
        ip: &str,
        remote_ip: &str,
        is_white: bool,
    ) -> (bool, UserInfo);

    fn check_white_list(&self, _key: &str) -> Option<UserId> {
        None
    }

    /// disable user if exists
    fn disable_user(&self, user_id: UserId);

    /// enable user if exists
    fn enable_user(&self, user_id: UserId);

    /// update user info if exists or insert a new one
    fn update_user_info(&self, user_info: UserInfo);

    /// update all
    fn update_from_json(&mut self, json: &str) {
        if let Ok(users) = serde_json::from_str::<Vec<UserInfo>>(json) {
            self.update_all(users);
        } else {
            error!("failed to parse json: {}", json);
        }
    }

    fn update_white_list(&mut self, white_list: Vec<WhiteListData>);

    fn update_all(&mut self, user_info: Vec<UserInfo>);

    fn in_stock(&self, ip: &str) -> bool;

    fn user_map_get(&self, remote_ip: &str) -> Option<UserInfo>;
}

pub struct DefaultAuthenticator;

impl Authenticator for DefaultAuthenticator {
    fn check_auth(
        &self,
        _username: &str,
        _password: &str,
        _ip: &str,
        _remote_ip: &str,
        _is_white: bool,
    ) -> (bool, UserInfo) {
        (true, UserInfo::default())
    }

    fn disable_user(&self, _user_id: UserId) {}

    fn enable_user(&self, _user_id: UserId) {}

    fn update_user_info(&self, _user_info: UserInfo) {}

    fn update_all(&mut self, _user_info: Vec<UserInfo>) {}

    fn update_white_list(&mut self, _white_list: Vec<WhiteListData>) {}

    fn in_stock(&self, _ip: &str) -> bool {
        false
    }

    fn user_map_get(&self, _remote_ip: &str) -> Option<UserInfo> {
        None
    }
}
