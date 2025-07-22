use ergo_chain_sync::client::model::BlockTransaction;

#[derive(Debug, Clone)]
pub enum LedgerTxEvent {
    AppliedTx {
        tx: BlockTransaction,
        timestamp: i64,
    },
    UnappliedTx(BlockTransaction),
}
