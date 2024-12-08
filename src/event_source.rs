use futures::stream::StreamExt;
use futures::{stream, Stream};
use std::sync::Arc;

use ergo_chain_sync::ChainUpgrade;
use ergo_mempool_sync::MempoolUpdate;

use log::info;

use crate::models::kafka_event::BlockEvent;
use crate::models::mempool_event::MempoolEvent;
use kafka::producer::{Producer, Record};

use crate::models::tx_event::TxEvent;
use async_std::task::spawn_blocking;

pub fn block_event_source<S>(
    upstream: S,
    producer: Producer,
    topic: String,
) -> impl Stream<Item = ChainUpgrade>
where
    S: Stream<Item = ChainUpgrade>,
{
    let topic = Arc::new(tokio::sync::Mutex::new(topic));
    let producer = Arc::new(std::sync::Mutex::new(producer));
    upstream.then(move |ev| {
        let topic = topic.clone();
        let producer = producer.clone();
        let ev_clone = ev.clone();
        async move {
            let block_event = BlockEvent::from(ev_clone);
            let block_id: String = match block_event.clone() {
                BlockEvent::BlockApply { id, .. } | BlockEvent::BlockUnapply { id, .. } => id,
            };
            let value = serde_json::to_string(&block_event).unwrap();
            let topic = topic.clone().lock().await.clone();
            spawn_blocking(move || {
                let rec: &Record<String, String> =
                    &Record::from_key_value(topic.as_str(), block_id.clone(), value.clone());
                info!("Block value is: ${:?}", value.clone());
                info!("Got new block. Key: ${:?}", block_id);
                producer.lock().unwrap().send(rec).unwrap();
                info!("New block processed by kafka. Key: ${:?}", block_id);
            })
            .await;
            ev
        }
    })
}

pub fn tx_event_source<S>(upstream: S) -> impl Stream<Item = TxEvent>
where
    S: Stream<Item = ChainUpgrade>,
{
    upstream.flat_map(|u| stream::iter(process_upgrade(u)))
}

fn process_upgrade(upgr: ChainUpgrade) -> Vec<TxEvent> {
    match upgr {
        ChainUpgrade::RollForward(blk) => blk
            .transactions
            .into_iter()
            .map(|tx| TxEvent::AppliedTx {
                tx,
                timestamp: blk.timestamp as i64,
                block_height: blk.height as i32,
            })
            .collect(),
        ChainUpgrade::RollBackward(blk) => {
            blk.transactions
                .into_iter()
                .rev() // we unapply txs in reverse order.
                .map(TxEvent::UnappliedTx)
                .collect()
        }
    }
}

pub fn mempool_event_source<S>(
    upstream: S,
    producer: Producer,
    topic: String,
) -> impl Stream<Item = ()>
where
    S: Stream<Item = MempoolUpdate>,
{
    let topic = Arc::new(tokio::sync::Mutex::new(topic));
    let producer = Arc::new(std::sync::Mutex::new(producer));
    upstream.then(move |event| {
        let topic = topic.clone();
        let producer = producer.clone();
        async move {
            let mempool_event = MempoolEvent::try_from(event.clone());
            if let Ok(kafka_event) = mempool_event {
                let kafka_string = serde_json::to_string(&kafka_event).unwrap();
                let tx_id: String = match event {
                    MempoolUpdate::TxAccepted(tx) => tx.id().into(),
                    MempoolUpdate::TxWithdrawn(tx) => tx.id().into(),
                    _ => "".to_string(),
                };

                let topic = topic.clone().lock().await.clone();
                spawn_blocking(move || {
                    let rec: &Record<String, String> =
                        &Record::from_key_value(topic.as_str(), tx_id.clone(), kafka_string);
                    info!("Got new mempool event. Key: ${:?}", tx_id);
                    producer.lock().unwrap().send(rec).unwrap();
                    info!("New mempool event processed by kafka. Key: ${:?}", tx_id);
                })
                .await;
            }
        }
    })
}
