use async_channel::Sender;
use tracing::error;
use rg_common::stat::StatData;
use tokio::sync::broadcast::Receiver;

#[derive(Debug)]
pub struct EmitClient {
    subscribes: Vec<Receiver<StatData>>,
    // better use trait
    io: Sender<StatData>,
}

impl EmitClient {
    pub fn new(io: Sender<StatData>) -> Self {
        Self {
            subscribes: Vec::new(),
            io,
        }
    }

    pub fn add_subscribe(&mut self, subscribe: Receiver<StatData>) {
        self.subscribes.push(subscribe);
    }

    pub async fn run(&mut self) {
        let (tx, rx) = async_channel::unbounded::<StatData>();
        // listen all subscription
        for subscribe in self.subscribes.iter_mut() {
            let tx = tx.clone();
            let mut subscribe = subscribe.resubscribe();
            tokio::spawn(async move {
                loop {
                    match subscribe.recv().await {
                        Ok(stat) => {
                            if let Err(e) = tx.send(stat).await {
                                error!("send stat error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("receive stat error: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        // send all stat to io
        loop {
            match rx.recv().await {
                Ok(stat) => {
                    if let Err(e) = self.io.send(stat).await {
                        error!("emit client emit stat error: {}", e);
                    }
                }
                Err(e) => {
                    error!("emit client receive stat error: {}", e);
                    break;
                }
            }
        }
    }
}
