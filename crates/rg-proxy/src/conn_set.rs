use std::collections::HashSet;

use dashmap::DashMap;
use tracing::info;
use rg_common::UserId;
use tokio::sync::broadcast::Sender;

pub(crate) struct ConnStat<T> {
    connections: DashMap<UserId, HashSet<usize>>,
    connection_map: DashMap<usize, T>,
}

#[allow(unused)]
impl ConnStat<Sender<()>> {
    pub(crate) fn kill_user(&self, user_id: UserId) {
        let conns = self.remove_all(user_id);
        for conn in conns {
            // send shutdown signal
            let _ = conn.send(());
        }
    }

    pub(crate) fn shutdown(&self) {
        info!("recieve shutdown signal, close all connections");
        for conns in self.connection_map.iter() {
            let _ = conns.value().send(());
        }
    }
}

impl<T> ConnStat<T> {
    pub(crate) fn new() -> Self {
        Self {
            connections: DashMap::new(),
            connection_map: DashMap::new(),
        }
    }

    pub(crate) fn add(&self, user_id: UserId, id: usize, sender: T) {
        self.connections
            .entry(user_id)
            .and_modify(|set| {
                set.insert(id);
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();
                set.insert(id);
                set
            });
        self.connection_map.insert(id, sender);
    }

    pub(crate) fn remove(&self, user_id: UserId, id: usize) {
        if let Some(mut set) = self.connections.get_mut(&user_id) {
            set.remove(&id);
        }
        self.connection_map.remove(&id);
    }

    #[allow(unused)]
    pub(crate) fn remove_all(&self, user_id: UserId) -> Vec<T> {
        if let Some((_, set)) = self.connections.remove(&user_id) {
            let mut vec = Vec::new();
            for id in set.into_iter() {
                if let Some((_, sender)) = self.connection_map.remove(&id) {
                    vec.push(sender);
                }
            }
            return vec;
        }
        vec![]
    }
}
