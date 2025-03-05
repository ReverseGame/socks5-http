use anyhow::anyhow;
use args::ENV_ARG;
use regex::Regex;
use rg_common::{Result, error::RgError};
use std::fmt::Display;
use std::sync::LazyLock;
use tokio::sync::OnceCell;
use error::Error;

pub static LOCAL_IP_CONFIG_FILE: LazyLock<&str> = LazyLock::new(|| match ENV_ARG.as_str() {
    "dev" => "/etc/dev_gre_tunnel_config",
    "beta" => "/etc/beta_gre_tunnel_config",
    "product" => "/etc/gre_tunnel_config",
    _ => "/etc/gre_tunnel_config",
});

const DEFAULT_PORT_START: u32 = 40000;
const DEFAULT_OFFSET: u32 = 2;

#[derive(Debug, Default)]
pub struct Config {
    pub local_ip: String,
    pub port_start: u32,
    pub port_end: u32,
    pub offset: u32,
    pub ip_range: Vec<IpRange>,
    pub extra_ips: Vec<String>,
    pub server_start: Option<String>,
    pub server_end: Option<String>,
}

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

pub static LOCAL_IPS: OnceCell<Vec<String>> = OnceCell::const_new();

pub async fn get_config() -> &'static Config {
    CONFIG
        .get_or_init(|| async {
            let data = tokio::fs::read_to_string(*LOCAL_IP_CONFIG_FILE)
                .await
                .unwrap();
            let ip_range = get_local_ip_range(&data).unwrap();
            let local_ip = get_field_ip(&data, "LOCAL_IP").unwrap();
            let port_start = get_port(&data, "PORT_START").unwrap_or(DEFAULT_PORT_START);
            let port_end = get_port(&data, "PORT_END").unwrap_or(port_start + 10000);
            let offset = get_port(&data, "OFFSET").unwrap_or(DEFAULT_OFFSET);
            let extra_ips = get_extra_ips(&data).unwrap();
            let server_start = get_field_ip(&data, "SERVER_START").ok();
            let server_end = get_field_ip(&data, "SERVER_END").ok();
            Config {
                local_ip,
                ip_range,
                port_start,
                port_end,
                offset,
                extra_ips,
                server_start,
                server_end,
            }
        })
        .await
}

pub async fn get_local_ip_port() -> &'static [String] {
    LOCAL_IPS
        .get_or_init(|| async {
            let config = get_config().await;
            let mut other = Vec::new();
            let mut ips = config
                .ip_range
                .iter()
                .flat_map(|x| {
                    let t = x.all_ips();
                    if config.offset != 0 {
                        let n = t.len();
                        other.push(t[0].clone());
                        other.push(t[1].clone());
                        other.push(t[n - 1].clone());
                        t[2..n - 1].to_vec()
                    } else {
                        t
                    }
                })
                .collect::<Vec<_>>();
            ips.extend(other);
            ips.extend(config.extra_ips.clone());
            let port_range = config.port_end.abs_diff(config.port_start) + 1;
            println!("port range: {}", port_range);
            let mut ind = 0;
            let ips = ips
                .iter()
                .map(|x| {
                    let ip = format!("{}:{}", x, config.port_start + ind);
                    ind = (ind + 1) % port_range;
                    ip
                })
                .collect::<Vec<_>>();
            ips
        })
        .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpRange {
    pub ip: String,
    pub mask: u8,
}

impl Display for IpRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.ip, self.mask)
    }
}

impl IpRange {
    #[allow(unused)]
    pub fn all_ips(&self) -> Vec<String> {
        let mut ips = Vec::new();
        let split = self
            .ip
            .split('.')
            .map(|x| x.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        assert!(split.len() == 4);

        // only consider the last 2 bytes
        for i in 0..(1 << (32 - self.mask)) {
            // add ip range
            let p4 = split[3] + i as u32;
            let p3 = split[2] + p4 / 256;
            let p4 = p4 % 256;
            let p3 = p3 % 256;
            let ip = format!("{}.{}.{}.{}", split[0], split[1], p3, p4);
            ips.push(ip);
        }
        ips
    }
}

/// Get the local ip range from the config file
pub fn get_local_ip_range(data: &str) -> Result<Vec<IpRange>> {
    let mut ip_prefixes = Vec::new();
    let reg = Regex::new(r"(\d+\.\d+\.\d+\.\d+)/(\d+)").unwrap();
    reg.captures_iter(data).for_each(|cap| {
        let ip = cap.get(1).unwrap().as_str();
        let mask = cap.get(2).unwrap().as_str();
        ip_prefixes.push(IpRange {
            ip: ip.to_string(),
            mask: mask.parse().unwrap(),
        });
    });
    Ok(ip_prefixes)
}

pub fn get_field_ip(data: &str, field: &str) -> Result<String> {
    let reg_str = format!(r#"{}=(\d+\.\d+\.\d+\.\d+)"#, field);
    let reg = Regex::new(&reg_str).unwrap();
    let caps = reg.captures(data);
    if let Some(cap) = caps {
        cap.get(1)
            .map(|c| c.as_str().to_string())
            .ok_or(anyhow!(format!("No field {} ip found", field)).into())
    } else {
        Err(RgError::from(anyhow!(format!(
            "No field {} ip found",
            field
        ))))
    }
}

pub fn get_port(data: &str, field: &str) -> Option<u32> {
    let reg_str = format!(r#"{}\s*=\s*(\d+)"#, field);
    let reg = Regex::new(&reg_str).unwrap();
    let caps = reg.captures(data);
    if let Some(cap) = caps {
        cap.get(1)
            .map(|c| c.as_str().parse::<u32>().unwrap_or(DEFAULT_PORT_START))
    } else {
        None
    }
}

pub fn get_extra_ips(data: &str) -> Result<Vec<String>> {
    let reg = Regex::new(r#""(\d+\.\d+\.\d+\.\d+)""#).unwrap();
    let mut extra_ips = Vec::new();
    reg.captures_iter(data).for_each(|cap| {
        let ip = cap.get(1).unwrap().as_str();
        extra_ips.push(ip.to_string());
    });
    Ok(extra_ips)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    pub fn get_resource_path() -> PathBuf {
        // get root path
        let mut path = std::env::current_dir().expect("fail to get current dir");
        loop {
            let mut p = path.clone();
            p.push("resources");
            if p.is_dir() {
                return p;
            }
            if !path.pop() {
                panic!("fail to get resource path");
            }
        }
    }

    #[tokio::test]
    async fn test_get_local_ip_range() {
        let mut path = get_resource_path();
        path.push("test/test_ip_config");
        let data = tokio::fs::read_to_string(path).await.unwrap();
        let ip_ranges = get_local_ip_range(&data).unwrap();
        assert_eq!(ip_ranges.len(), 1);
        let ip_range = IpRange {
            ip: "156.239.16.0".to_string(),
            mask: 21,
        };
        assert_eq!(ip_ranges[0], ip_range);
        let all_ips = ip_range.all_ips();
        assert_eq!(all_ips.len(), 2048);
    }

    #[tokio::test]
    async fn test_ip_range() {
        let ip_range = IpRange {
            ip: "156.239.16.0".to_string(),
            mask: 24,
        };
        let all_ips = ip_range.all_ips();
        assert_eq!(all_ips.len(), 256);

        let ip_range = IpRange {
            ip: "181.215.18.10".to_string(),
            mask: 24,
        };
        let all_ip = ip_range.all_ips();
        println!("{:?}", all_ip);
    }

    #[tokio::test]
    async fn test_get_local_ip() {
        let mut path = get_resource_path();
        path.push("test/test_ip_config");
        let data = tokio::fs::read_to_string(path).await.unwrap();
        let ip = get_field_ip(&data, "LOCAL_IP").unwrap();
        println!("{}", ip);
        let server_start = get_field_ip(&data, "SERVER_START").ok();
        let server_end = get_field_ip(&data, "SERVER_END").ok();
        println!("{:?}, {:?}", server_start, server_end);
    }

    #[tokio::test]
    async fn test_get_port() {
        let mut path = get_resource_path();
        path.push("test/test_ip_config");
        let data = tokio::fs::read_to_string(path).await.unwrap();
        let port = get_port(&data, "PORT_START");
        assert!(port.is_some());
        assert_eq!(port, Some(40000));
        let port = get_port(&data, "PORT_END");
        assert!(port.is_some());
        assert_eq!(port, Some(40000));
        let offset = get_port(&data, "OFFSET");
        assert_eq!(offset, Some(0));
    }

    #[tokio::test]
    async fn test_get_extra_ips() {
        let mut path = get_resource_path();
        path.push("test/test_ip_config");
        let data = tokio::fs::read_to_string(path).await.unwrap();
        let extras = get_extra_ips(&data).unwrap();
        println!("{:?}", extras);
    }
}
