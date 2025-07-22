use async_std::task::spawn_blocking;
use base64::{engine::general_purpose, Engine as _};
use spectrum_offchain::event_sink::types::EventHandler;
use std::sync::Arc;

use kafka::producer::{Producer, Record};

use async_trait::async_trait;
use log::info;
use serde_json::json;

use crate::models::cbor::CborBlockTransaction;
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
            let tx_id: String = match &ev_clone {
                TxEvent::AppliedTx { tx, .. } | TxEvent::UnappliedTx { tx, .. } => tx.id.into(),
            };

            let kafka_json = match ev_clone.clone() {
                TxEvent::AppliedTx {
                    timestamp,
                    tx,
                    block_height,
                    block_id,
                } => {
                    let cbor_tx = CborBlockTransaction::from(tx);
                    let tx_bytes = serde_cbor::to_vec(&cbor_tx).unwrap();
                    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);
                    json!({
                        "AppliedEvent": {
                            "timestamp": timestamp,
                            "height": block_height,
                            "tx": tx_base64,
                            "block_id": block_id,
                        }
                    })
                }
                TxEvent::UnappliedTx {
                    timestamp,
                    tx,
                    block_height,
                    block_id,
                } => {
                    let cbor_tx = CborBlockTransaction::from(tx);
                    let tx_bytes = serde_cbor::to_vec(&cbor_tx).unwrap();
                    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);
                    json!({
                        "UnappliedEvent": {
                            "timestamp": timestamp,
                            "height": block_height,
                            "tx": tx_base64,
                            "block_id": block_id,
                        }
                    })
                }
            };
            let kafka_string = kafka_json.to_string();

            let topic = topic.clone().lock().await.clone();
            spawn_blocking(move || {
                let rec: &Record<String, String> =
                    &Record::from_key_value(topic.as_str(), tx_id.clone(), kafka_string);
                let event_type = match ev_clone {
                    TxEvent::AppliedTx { .. } => "AppliedTx",
                    TxEvent::UnappliedTx { .. } => "UnappliedTx",
                };
                info!("Got new event. Type: {}, Key: ${:?}", event_type, tx_id);
                producer.lock().unwrap().send(rec).unwrap();
                info!("New event processed by kafka. Key: ${:?}", tx_id);
            })
            .await;
            Some(ev)
        }
        .await
    }
}
