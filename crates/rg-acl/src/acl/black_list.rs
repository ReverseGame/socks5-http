use std::collections::HashSet;

use super::AclRule;

#[derive(Default, Debug)]
pub struct BlackListAclRule {
    host: HashSet<String>,
    user_host: HashSet<String>,
    ip_host: HashSet<String>,
}

// simple match implementation
// may need regex in the future
impl AclRule for BlackListAclRule {
    fn check(&self, user_info: &rg_common::user_auth::UserInfo, host: &str, ip: &str) -> bool {
        if self.host.contains(host) {
            return false;
        }

        if self
            .user_host
            .contains(&format!("{}-{}", user_info.user_id, host))
        {
            return false;
        }

        if self.ip_host.contains(&format!("{}-{}", ip, host)) {
            return false;
        }

        true
    }

    fn update(&mut self, _data: &str) {}
}
