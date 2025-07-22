use ergo_chain_sync::model::Block;
use ergo_chain_sync::ChainUpgrade;
use serde::{Deserialize, Serialize};

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
