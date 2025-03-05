use async_channel::{Receiver, Sender};
use config::{CONFIG, KafkaConfig};
use error::Result;
use tracing::{error, info};
use rdkafka::{
    ClientConfig, Message,
    admin::{AdminClient, AdminOptions},
    consumer::{Consumer, DefaultConsumerContext, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use serde::Serialize;
use tokio::sync::OnceCell;
use uuid::Uuid;

static KAFKA_MANAGER: OnceCell<KafkaManager> = OnceCell::const_new();

const DEFAULT_GROUP_ID: &str = "rg-server-consumer-";

type DefaultConsumer = StreamConsumer<DefaultConsumerContext>;

pub async fn get_kafka_manager() -> &'static KafkaManager {
    KAFKA_MANAGER
        .get_or_init(|| async { KafkaManager::new(CONFIG.kafka_config.clone()) })
        .await
}

pub struct KafkaManager {
    group_id: String,
    config: KafkaConfig,

    close_sender: Sender<()>,
    close_receiver: Receiver<()>,
}

impl KafkaManager {
    pub fn new(config: KafkaConfig) -> Self {
        let (sender, receiver) = async_channel::bounded::<()>(1);
        Self {
            config,
            group_id: format!("{}-{}", DEFAULT_GROUP_ID, Uuid::new_v4()),
            close_sender: sender,
            close_receiver: receiver,
        }
    }

    pub async fn close(&self) {
        info!("close kafka manager...");
        let _ = self.close_sender.send(()).await;
        self.remove_consumer_group().await;
    }

    pub fn create_client_config(&self) -> ClientConfig {
        let mut client_config = ClientConfig::new();
        client_config
            .set("group.id", self.group_id.as_str())
            .set("bootstrap.servers", self.config.brokers.join(","))
            .set("enable.auto.commit", "true")
            .set("allow.auto.create.topics", "true")
            .set_log_level(rdkafka::config::RDKafkaLogLevel::Error);
        if let (Some(username), Some(password)) =
            (self.config.username.as_ref(), self.config.password.as_ref())
        {
            client_config
                .set("sasl.mechanisms", "SCRAM-SHA-256")
                .set("security.protocol", "SASL_PLAINTEXT")
                .set("sasl.username", username)
                .set("sasl.password", password);
        }
        client_config
    }

    pub fn create_consumer(&self, topic: &str) -> Result<Receiver<String>> {
        let consumer: DefaultConsumer = self
            .create_client_config()
            .create_with_context(DefaultConsumerContext {})?;
        consumer.subscribe(&[topic])?;
        let (sender, receiver) = async_channel::unbounded::<String>();
        let close = self.close_receiver.clone();
        tokio::spawn(async move {
            loop {
                while let Ok(msg) = consumer.recv().await {
                    let payload = match msg.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            error!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };
                    if !payload.is_empty() {
                        let _ = sender.send(payload.to_string()).await;
                    }
                    if close.try_recv().is_ok() {
                        info!("receive close signal, close kafka consumer...");
                        consumer.unsubscribe();
                        break;
                    }
                }
            }
        });
        Ok(receiver)
    }

    pub fn create_producer(&self) -> FutureProducer {
        let producer: FutureProducer = self
            .create_client_config()
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer create failed");
        producer
    }

    pub async fn remove_consumer_group(&self) {
        info!("remove kafka consumer group: {}", self.group_id);
        let admin_client: AdminClient<_> = self
            .create_client_config()
            .create()
            .expect("AdminClient create failed");
        let admin_option = AdminOptions::default();
        let _ = admin_client
            .delete_groups(&[self.group_id.as_str()], &admin_option)
            .await;
    }
}

pub async fn start_msg_send_job<T>(receiver: Receiver<T>, topic: &str)
where
    T: Serialize + Send,
{
    info!("start send traffic info to kafka...");
    loop {
        let producer = get_kafka_manager().await.create_producer();
        while let Ok(info) = receiver.recv().await {
            if let Ok(payload) = serde_json::to_string(&info) {
                for _ in 0..3 {
                    let record: FutureRecord<'_, String, String> =
                        FutureRecord::to(topic).payload(&payload);
                    if let Err((e, _msg)) = producer.send(record, Timeout::Never).await {
                        error!("send traffic info to kafka failed: {:?}", e);
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
