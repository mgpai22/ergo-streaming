use ergo_lib::ergo_chain_types::BlockId;
use serde::{Deserialize, Serialize};

use crate::client::model::{BlockTransaction, FullBlock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub id: BlockId,
    pub parent_id: BlockId,
    pub height: u32,
    pub timestamp: u64,
    pub transactions: Vec<BlockTransaction>,
}

impl From<FullBlock> for Block {
    fn from(fb: FullBlock) -> Self {
        Self {
            id: fb.header.id,
            parent_id: fb.header.parent_id,
            height: fb.header.height,
            timestamp: fb.header.timestamp,
            transactions: fb.transactions,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BlockRecord {
    pub id: BlockId,
    pub height: u32,
}
