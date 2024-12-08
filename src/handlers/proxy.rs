use async_std::task::spawn_blocking;
use spectrum_offchain::event_sink::types::EventHandler;
use std::sync::Arc;

use kafka::producer::{Producer, Record};

use crate::models::kafka_event::KafkaEvent;
use async_trait::async_trait;
use log::info;

use crate::models::tx_event::TxEvent;

pub struct ProxyEvents {
    pub producer: Arc<std::sync::Mutex<Producer>>,
    pub topic: String,
}

impl ProxyEvents {
    pub fn new(producer: Arc<std::sync::Mutex<Producer>>, topic: String) -> Self {
        Self { producer, topic }
    }
}

#[async_trait(? Send)]
impl EventHandler<TxEvent> for ProxyEvents {
    async fn try_handle(&mut self, ev: TxEvent) -> Option<TxEvent> {
        let topic = Arc::new(tokio::sync::Mutex::new(self.topic.clone()));
        let producer = self.producer.clone();

        let ev_clone = ev.clone();
        async move {
            let kafka_event = KafkaEvent::from(ev_clone.clone());
            let tx_id: String = match ev_clone {
                TxEvent::AppliedTx { tx, .. } | TxEvent::UnappliedTx(tx) => tx.id().into(),
            };
            let kafka_string = serde_json::to_string(&kafka_event).unwrap();
            let topic = topic.clone().lock().await.clone();
            spawn_blocking(move || {
                let rec: &Record<String, String> =
                    &Record::from_key_value(topic.as_str(), tx_id.clone(), kafka_string);
                info!("Got new event. Key: ${:?}", tx_id);
                producer.lock().unwrap().send(rec).unwrap();
                info!("New event processed by kafka. Key: ${:?}", tx_id);
            })
            .await;
            Some(ev)
        }
        .await
    }
}
