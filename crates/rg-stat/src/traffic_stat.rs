use std::{collections::HashMap, sync::atomic::AtomicU64};

use dashmap::DashMap;
use rg_common::{stat::TrafficTotalStatSnapshot, TrafficInfo, UserId};

use crate::StatCollectable;

pub struct TrafficTotalStat {
    pub total: AtomicU64,
    pub upload: AtomicU64,
    pub download: AtomicU64,
}

impl StatCollectable for TrafficTotalStat {
    fn stat_type(&self) -> crate::StatType {
        crate::StatType::TrafficTotal
    }

    fn _collect(&mut self) -> String {
        let snap = TrafficTotalStatSnapshot {
            total: self
                .total
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            upload: self
                .upload
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
            download: self
                .download
                .fetch_and(0, std::sync::atomic::Ordering::Relaxed),
        };
        serde_json::to_string(&snap).unwrap_or_default()
    }
}

impl TrafficTotalStat {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            upload: AtomicU64::new(0),
            download: AtomicU64::new(0),
        }
    }

    pub fn add(&self, upload: u64, download: u64) {
        self.total
            .fetch_add(upload + download, std::sync::atomic::Ordering::Relaxed);
        self.upload
            .fetch_add(upload, std::sync::atomic::Ordering::Relaxed);
        self.download
            .fetch_add(download, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct TrafficUserStat {
    pub traffic_info: DashMap<UserId, HashMap<String, TrafficInfo>>,
}

impl StatCollectable for TrafficUserStat {
    fn stat_type(&self) -> crate::StatType {
        crate::StatType::UserTraffic
    }

    fn _collect(&mut self) -> String {
        let mut res = vec![];
        for infos in self.traffic_info.iter() {
            for (_, info) in infos.iter() {
                res.push(info.clone());
            }
        }
        if let Ok(msg) = serde_json::to_string(&res) {
            self.traffic_info.clear();
            msg
        } else {
            "".to_string()
        }
    }
}

impl TrafficUserStat {
    pub fn new() -> Self {
        Self {
            traffic_info: DashMap::new(),
        }
    }

    pub fn add(&self, info: &TrafficInfo) {
        // let user_id = info.user_id;
        let key = info.get_key();
        // update traffic info or insert
        self.traffic_info
            .entry(info.user_id)
            .and_modify(|old| {
                old.entry(key.clone())
                    .and_modify(|old| {
                        old.upload += info.upload;
                        old.download += info.download;
                    })
                    .or_insert(info.clone());
            })
            .or_insert_with(|| {
                let mut map = HashMap::new();
                map.insert(key, info.clone());
                map
            });
    }
}
