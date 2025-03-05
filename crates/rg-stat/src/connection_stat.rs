use std::sync::atomic::AtomicI64;

use rg_common::stat::ConnectionStatSnapshot;

use crate::StatCollectable;

pub struct ConnectionStat {
    pub alive_in_connection: AtomicI64,
    // pub alive_out_connection: AtomicI64,
}

impl StatCollectable for ConnectionStat {
    fn stat_type(&self) -> crate::StatType {
        crate::StatType::Connection
    }

    fn _collect(&mut self) -> String {
        let snap = ConnectionStatSnapshot {
            alive_in_connection: self
                .alive_in_connection
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            // alive_out_connection: self
            //     .alive_out_connection
            //     .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
        };
        serde_json::to_string(&snap).unwrap_or_default()
    }
}

impl ConnectionStat {
    pub fn new() -> Self {
        Self {
            alive_in_connection: AtomicI64::new(0),
            // alive_out_connection: AtomicI64::new(0),
        }
    }

    pub fn add(&self, in_conn: i64) {
        self.alive_in_connection
            .fetch_add(in_conn, std::sync::atomic::Ordering::Relaxed);
        // self.alive_out_connection
        //     .fetch_add(out_conn, std::sync::atomic::Ordering::Relaxed);
    }
}
