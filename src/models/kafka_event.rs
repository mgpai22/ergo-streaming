use ergo_lib::ergotree_ir::serialization::SigmaSerializable;

use base64::{engine::general_purpose, Engine as _};
use ergo_chain_sync::model::Block;
use ergo_chain_sync::ChainUpgrade;

use crate::models::tx_event::TxEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum KafkaEvent {
    AppliedEvent {
        timestamp: i64,
        tx: String,
        height: i32,
    },
    UnappliedEvent(String),
}

impl From<TxEvent> for KafkaEvent {
    fn from(value: TxEvent) -> Self {
        match value {
            TxEvent::AppliedTx {
                timestamp,
                tx,
                block_height,
            } => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                KafkaEvent::AppliedEvent {
                    timestamp,
                    tx: encoded,
                    height: block_height,
                }
            }
            TxEvent::UnappliedTx(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                KafkaEvent::UnappliedEvent(encoded)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockEvent {
    BlockApply {
        timestamp: u64,
        height: u32,
        id: String,
        num_txs: usize,
    },
    BlockUnapply {
        timestamp: u64,
        height: u32,
        id: String,
        num_txs: usize,
    },
}

impl From<ChainUpgrade> for BlockEvent {
    fn from(value: ChainUpgrade) -> Self {
        match value {
            ChainUpgrade::RollForward(Block {
                id,
                parent_id: _,
                height,
                timestamp,
                transactions,
            }) => {
                let id: String = base16::encode_lower(id.0 .0.as_ref());
                BlockEvent::BlockApply {
                    timestamp,
                    height,
                    id,
                    num_txs: transactions.len(),
                }
            }
            ChainUpgrade::RollBackward(Block {
                id,
                parent_id: _,
                height,
                timestamp,
                transactions,
            }) => {
                let id: String = base16::encode_lower(id.0 .0.as_ref());
                BlockEvent::BlockUnapply {
                    timestamp,
                    height,
                    id,
                    num_txs: transactions.len(),
                }
            }
        }
    }
}
