use base64::engine::general_purpose;
use base64::Engine;
use ergo_mempool_sync::MempoolUpdate;
use log::info;
use serde::{Deserialize, Serialize};

use crate::models::cbor::CborBlockTransaction;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MempoolEvent {
    TxAccepted { tx: String },
    TxWithdrawn { tx: String, confirmed: bool },
}

impl TryFrom<MempoolUpdate> for MempoolEvent {
    type Error = ();

    fn try_from(value: MempoolUpdate) -> Result<Self, Self::Error> {
        match value {
            MempoolUpdate::TxAccepted(tx) => {
                let cbor_tx = CborBlockTransaction::from(tx);
                let tx_bytes = serde_cbor::to_vec(&cbor_tx).unwrap();
                let encoded: String = general_purpose::STANDARD.encode(tx_bytes);
                Ok(MempoolEvent::TxAccepted { tx: encoded })
            }
            MempoolUpdate::TxWithdrawn(tx) => {
                info!(target: "mempool_event", "TxWithdrawn: {}", tx.id.to_string());
                let cbor_tx = CborBlockTransaction::from(tx);
                let tx_bytes = serde_cbor::to_vec(&cbor_tx).unwrap();
                let encoded: String = general_purpose::STANDARD.encode(tx_bytes);
                Ok(MempoolEvent::TxWithdrawn {
                    tx: encoded,
                    confirmed: false,
                })
            }
            MempoolUpdate::TxConfirmed(tx) => {
                info!(target: "mempool_event", "TxConfirmed: {}", tx.id.to_string());
                let cbor_tx = CborBlockTransaction::from(tx);
                let tx_bytes = serde_cbor::to_vec(&cbor_tx).unwrap();
                let encoded: String = general_purpose::STANDARD.encode(tx_bytes);
                Ok(MempoolEvent::TxWithdrawn {
                    tx: encoded,
                    confirmed: true,
                })
            }
        }
    }
}
