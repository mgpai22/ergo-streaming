use base64::engine::general_purpose;
use base64::Engine;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_mempool_sync::MempoolUpdate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MempoolEvent {
    TxAccepted { tx: String },
    TxWithdrawn { tx: String },
}

impl TryFrom<MempoolUpdate> for MempoolEvent {
    type Error = ();

    fn try_from(value: MempoolUpdate) -> Result<Self, Self::Error> {
        match value {
            MempoolUpdate::TxAccepted(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                Ok(MempoolEvent::TxAccepted { tx: encoded })
            }
            MempoolUpdate::TxWithdrawn(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                Ok(MempoolEvent::TxWithdrawn { tx: encoded })
            }
            _ => Err(()),
        }
    }
}
