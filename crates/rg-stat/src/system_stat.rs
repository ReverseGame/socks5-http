use std::time::Duration;

use pinger::{ping, PingOptions, PingResult};
use rg_common::stat::{StatType, SystemStatSnapshot};
use sysinfo::{Networks, System};

use crate::StatCollectable;

pub struct SystemStat {
    network: Networks,
}

impl StatCollectable for SystemStat {
    fn stat_type(&self) -> StatType {
        StatType::System
    }

    fn _collect(&mut self) -> String {
        let mut sys = System::new_all();

        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32;

        self.network.refresh(false);
        let (recv, transmit) = self
            .network
            .iter()
            .fold((0, 0), |(recv, transmit), (_, data)| {
                (recv + data.received(), transmit + data.transmitted())
            });
        let ping_latency = self.ping_latency();
        let snap = SystemStatSnapshot {
            cpu_usage,
            memory_usage,
            network_receive: recv,
            network_transmit: transmit,
            ping_latency,
        };
        serde_json::to_string(&snap).unwrap()
    }
}

impl SystemStat {
    pub fn new() -> Self {
        Self {
            network: Networks::new_with_refreshed_list(),
        }
    }
    fn ping_latency(&self) -> Option<f32> {
        let opt = PingOptions::new("google.com", Duration::from_secs(1), None);
        if let Ok(stream) = ping(opt) {
            for (i, res) in stream.iter().enumerate() {
                if i > 3 {
                    return None;
                }
                if let PingResult::Pong(rtt, _) = res {
                    return Some(rtt.as_millis() as f32);
                }
            }
        }
        None
    }
}
