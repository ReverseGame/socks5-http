mod black_list;

use rg_common::user_auth::UserInfo;

pub trait AclRule {
    fn check(&self, user_info: &UserInfo, host: &str, ip: &str) -> bool;

    fn update(&mut self, data: &str);
}

pub struct DefaultAclRule;

impl AclRule for DefaultAclRule {
    fn check(&self, _user_info: &UserInfo, _host: &str, _ip: &str) -> bool {
        true
    }

    fn update(&mut self, _data: &str) {}
}
