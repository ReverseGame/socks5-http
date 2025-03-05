use std::collections::HashSet;

use dashmap::DashMap;
use rg_common::{
    user_auth::{UserInfo, WhiteListData},
    UserId,
};

use crate::auth::Authenticator;

pub const IP: &str = "IP";
pub const PASSWORD: &str = "PASSWORD";

#[derive(Default, Debug)]
pub struct DcAuthenticator {
    // whitelist: ip-username-password => UserId, if default user, user_id = 0
    user_white_list: DashMap<String, UserId>,

    // user_id -> user_info
    pub user_map: DashMap<UserId, UserInfo>,

    // ip + username -> user_id
    ip_map: DashMap<String, UserId>,

    in_stock: HashSet<String>,
}

impl Authenticator for DcAuthenticator {
    fn check_white_list(&self, key: &str) -> Option<UserId> {
        if let Some(user_id) = self.user_white_list.get(key) {
            return Some(*user_id);
        }
        None
    }

    fn check_auth(
        &self,
        username: &str,
        password: &str,
        ip: &str,
        remote_ip: &str,
        is_white: bool,
    ) -> (bool, UserInfo) {
        // check white list
        if is_white {
            if let Some(user_info) = self.user_map_get(remote_ip) {
                if user_info.available
                    && user_info.auth_type == IP
                    && remote_ip == user_info.white_ip
                {
                    return (true, UserInfo::clone_id(&user_info));
                }
            }
        } else if let Some(user_id) =
            self.check_white_list(&format!("{}-{}-{}", ip, username, password))
        {
            if user_id == 0 {
                return (true, UserInfo::default());
            }
            if let Some(user_info) = self.user_map.get(&user_id) {
                if user_info.available && user_info.auth_type == PASSWORD {
                    return (true, UserInfo::clone_id(&user_info));
                }
            }
        }
        // check user password
        let key = if username.is_empty() {
            remote_ip.to_string()
        } else {
            format!("{}-{}", ip, username)
        };
        if let Some(user_id) = self.ip_map.get(&key) {
            if let Some(user_info) = self.user_map.get(&user_id) {
                if user_info.password == password && user_info.available {
                    return (true, UserInfo::clone_id(&user_info));
                }
            }
        }
        (false, UserInfo::default())
    }

    fn disable_user(&self, user_id: UserId) {
        self.user_map.entry(user_id).and_modify(|info| {
            info.available = false;
        });
    }

    fn enable_user(&self, user_id: UserId) {
        self.user_map.entry(user_id).and_modify(|info| {
            info.available = true;
        });
    }

    fn update_user_info(&self, user_info: UserInfo) {
        self.update_ip_map(&user_info);
        self.user_map
            .entry(user_info.user_id)
            .and_modify(|info| info.clone_from(&user_info))
            .or_insert(user_info);
    }

    fn update_all(&mut self, user_infos: Vec<UserInfo>) {
        let user_map = DashMap::new();
        let ip_map = DashMap::new();
        for user in user_infos {
            for ip in &user.ips {
                let key = if user.auth_type == IP {
                    user.white_ip.to_string()
                } else {
                    format!("{}-{}", ip, user.username)
                };
                ip_map.insert(key, user.user_id);
            }
            user_map.insert(user.user_id, user);
        }
        self.user_map = user_map;
        self.ip_map = ip_map;
    }

    fn update_white_list(&mut self, white_list: Vec<WhiteListData>) {
        let user_white_list = DashMap::new();
        let mut in_stock = HashSet::new();
        for white in white_list {
            let key = if white.username.is_empty() && white.password.is_empty() {
                white.ip.to_string()
            } else {
                format!("{}-{}-{}", white.ip, white.username, white.password)
            };
            user_white_list.insert(key, white.user_id);
            in_stock.insert(white.ip.clone());
        }
        self.user_white_list = user_white_list;
        self.in_stock = in_stock;
    }

    fn in_stock(&self, ip: &str) -> bool {
        self.in_stock.contains(ip)
    }

    fn user_map_get(&self, remote_ip: &str) -> Option<UserInfo> {
        if let Some(user_id) = self.ip_map.get(remote_ip) {
            self.user_map.get(user_id.value()).map(|info| info.clone())
        } else {
            None
        }
    }
}

impl DcAuthenticator {
    fn update_ip_map(&self, user_info: &UserInfo) {
        for ip in &user_info.ips {
            let key = if user_info.username.is_empty() {
                user_info.white_ip.to_string()
            } else {
                format!("{}-{}", ip, user_info.username)
            };
            self.ip_map
                .entry(key)
                .and_modify(|old| *old = user_info.user_id)
                .or_insert(user_info.user_id);
        }
    }
}
