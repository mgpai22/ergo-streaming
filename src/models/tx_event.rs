use ergo_chain_sync::client::model::BlockTransaction;

/// Possible events that can happen with transactions on-chain.
#[derive(Debug, Clone)]
pub enum TxEvent {
    AppliedTx {
        timestamp: i64,
        tx: BlockTransaction,
        block_height: i32,
        block_id: String,
    },
    UnappliedTx {
        timestamp: i64,
        tx: BlockTransaction,
        block_height: i32,
        block_id: String,
    },
}
