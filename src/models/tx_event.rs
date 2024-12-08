use ergo_lib::chain::transaction::Transaction;

/// Possible events that can happen with transactions on-chain.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxEvent {
    AppliedTx {
        timestamp: i64,
        tx: Transaction,
        block_height: i32,
    },
    UnappliedTx(Transaction),
}
