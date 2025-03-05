use std::collections::HashMap;

use chrono::Utc;
use tracing::{error, info};
use rg_common::{
    stat::{StatData, StatType},
    TrafficInfo,
};
use tokio::sync::{broadcast::Sender, mpsc::UnboundedReceiver, Mutex};

pub use request_stat::RequestType;
mod connection_stat;
mod request_stat;
mod system_stat;
mod traffic_stat;

pub struct StatisticManager {
    total_traffic: traffic_stat::TrafficTotalStat,
    user_traffic: Mutex<traffic_stat::TrafficUserStat>,
    connection_stat: connection_stat::ConnectionStat,
    request_stat: request_stat::RequestStat,
    system_stat: system_stat::SystemStat,
    subscribed: HashMap<StatType, Sender<StatData>>,
    listener: UnboundedReceiver<StatEvent>,
}

impl StatisticManager {
    pub fn new(listener: UnboundedReceiver<StatEvent>) -> Self {
        Self {
            total_traffic: traffic_stat::TrafficTotalStat::new(),
            user_traffic: Mutex::new(traffic_stat::TrafficUserStat::new()),
            connection_stat: connection_stat::ConnectionStat::new(),
            request_stat: request_stat::RequestStat::new(),
            system_stat: system_stat::SystemStat::new(),
            subscribed: HashMap::with_capacity(4),
            listener,
        }
    }

    pub fn subscribe(&mut self, stat_type: StatType) -> tokio::sync::broadcast::Receiver<StatData> {
        if self.subscribed.contains_key(&stat_type) {
            return self.subscribed[&stat_type].subscribe();
        }
        let (tx, rx) = tokio::sync::broadcast::channel(500);
        self.subscribed.insert(stat_type, tx);
        rx
    }

    pub async fn collect_stat(&mut self) {
        for (stat_type, tx) in &mut self.subscribed {
            let stat = match stat_type {
                StatType::UserTraffic => {
                    let mut user_stat = self.user_traffic.lock().await;
                    user_stat.collect()
                }
                StatType::TrafficTotal => self.total_traffic.collect(),
                StatType::Request => self.request_stat.collect(),
                StatType::Connection => self.connection_stat.collect(),
                StatType::System => self.system_stat.collect(),
            };
            if stat.data.is_empty() {
                continue;
            }
            // TODO: if capacity is full will get error, TBD: how to handle this error
            if let Err(e) = tx.send(stat) {
                error!("send stat error: {}", e);
            }
        }
    }

    pub async fn run(&mut self) {
        let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(60));
        timer.reset();
        loop {
            tokio::select! {
                _ = timer.tick() => {
                    info!("try collect stat");
                    self.collect_stat().await;
                    timer.reset();
                }
                event = self.listener.recv() => {
                    if let Some(event) = event {
                        match event {
                            StatEvent::Traffic(info) => {
                                self.total_traffic.add(info.upload, info.download);
                                let user_stat = self.user_traffic.lock().await;
                                user_stat.add(&info);
                            }
                            StatEvent::Request(t) => {
                                self.request_stat.add(t);
                            }
                            StatEvent::Connection(in_cnt) => {
                                self.connection_stat.add(in_cnt);
                            }
                        }
                    }
                }
            }
        }
    }
}

trait StatCollectable {
    fn stat_type(&self) -> StatType;
    fn _collect(&mut self) -> String;

    fn collect(&mut self) -> StatData {
        StatData {
            stat_type: self.stat_type(),
            data: self._collect(),
            timestamp: Utc::now().timestamp() as u64,
        }
    }
}

#[derive(Debug)]
pub enum StatEvent {
    Traffic(TrafficInfo),
    Request(RequestType),
    Connection(i64),
}
