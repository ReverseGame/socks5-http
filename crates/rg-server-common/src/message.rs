use rg_common::{
    stat::StatData,
    user_auth::{UserInfo, WhiteListData},
    TrafficInfo, UserId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ServerIpInfo {
    pub local_ip: String,
    pub ip_range: Vec<String>,
    pub port_start: u32,
    pub port_end: Option<u32>,
    // offset for port range
    pub offset: Option<u32>,
    pub extra_ips: Vec<String>,
    pub server_start: Option<String>,
    pub server_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserTrafficInfo {
    pub user_traffics: Vec<TrafficInfo>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientMessage {
    Authenticate(String),
    ClientInfoStat(StatData),
    UserTrafficStat(UserTrafficInfo),
    IpRange(ServerIpInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerMessage {
    // all acl data
    AclData(String),
    // all user auth data for client ip range
    UserAuth(Vec<UserInfo>),

    UserWhiteList(Vec<WhiteListData>),
    // update user auth data
    UpdateUser(UserInfo),
    // disable user
    DisableUser(UserId),
}

#[cfg(test)]
mod test {
    use std::vec;

    use rg_common::stat::StatData;

    use crate::auth::AUTH_PRIVATE_KEY;

    #[test]
    fn test_enum_serde() {
        let stat_data = StatData {
            stat_type: rg_common::stat::StatType::Connection,
            data: "test".to_string(),
            timestamp: 123,
        };
        let msg = super::ClientMessage::ClientInfoStat(stat_data);
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            json,
            r#"{"ClientInfoStat":{"stat_type":"Connection","data":"test","timestamp":123}}"#
        );
        let msg_t: super::ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, msg_t);

        let msg = super::ClientMessage::Authenticate(AUTH_PRIVATE_KEY.to_string());
        let json = serde_json::to_string(&msg).unwrap();
        println!("{}", json);

        let data = super::ServerIpInfo {
            local_ip: "192.168.0.1".to_string(),
            ip_range: vec!["152.168.0.0/21".to_string()],
            port_start: 40000,
            port_end: Some(40000),
            extra_ips: vec![],
            ..Default::default()
        };
        let msg = super::ClientMessage::IpRange(data);
        let json = serde_json::to_string(&msg).unwrap();
        println!("{}", json);
    }
}
