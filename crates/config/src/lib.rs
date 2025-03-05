use tracing::info;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

pub fn init_config() -> Config {
    info!("{:?}", CONFIG.clone());
    CONFIG.clone()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub redis_config: RedisConfig,
    pub kafka_config: KafkaConfig,
    pub database_config: DatabaseConfig,
}

impl Config {
    /// load config from resource folder
    pub fn load() -> Self {
        let env = env::var("env").unwrap_or("dev".to_string());
        let mut base_path = Self::get_resource_path();
        base_path.push(format!("config/server_config_{}.yaml", env));
        serde_yaml::from_str(&std::fs::read_to_string(base_path).expect("fail to read config file"))
            .expect("parse config file error")
    }

    /// get resources folder path
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub addr: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: String,
}

impl RedisConfig {
    pub fn connection_string(&self) -> String {
        let username_part = self.username.as_deref().unwrap_or_default();
        let password_part = self
            .password
            .as_ref()
            .map(|p| format!(":{}@", p))
            .unwrap_or_default();
        let auth = if !username_part.is_empty() || !password_part.is_empty() {
            format!("{}{}", username_part, password_part)
        } else {
            "".to_string()
        };
        format!("redis://{}{}/{}", auth, self.addr, self.db)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub traffic_topic: String,
    pub ealry_stop_topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub addr: String,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "mysql://{}:{}@{}/{}",
            self.username, self.password, self.addr, self.database
        )
    }
}
