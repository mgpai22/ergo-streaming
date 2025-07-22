use ergo_chain_sync::client::model::BlockTransaction;
use ergo_lib::chain::transaction::{DataInput, TxId};
use ergo_lib::ergotree_ir::chain::ergo_box::{BoxId, ErgoBox, NonMandatoryRegisters};
use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CborToken {
    token_id: TokenId,
    amount: u64,
}

impl From<Token> for CborToken {
    fn from(t: Token) -> Self {
        Self {
            token_id: t.token_id,
            amount: *t.amount.as_u64(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CborDataInput {
    box_id: BoxId,
}

impl From<DataInput> for CborDataInput {
    fn from(di: DataInput) -> Self {
        Self { box_id: di.box_id }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CborErgoBox {
    box_id: BoxId,
    value: u64,
    ergo_tree: String,
    assets: Vec<CborToken>,
    additional_registers: NonMandatoryRegisters,
    creation_height: u32,
    transaction_id: TxId,
    index: u16,
}

impl From<ErgoBox> for CborErgoBox {
    fn from(b: ErgoBox) -> Self {
        Self {
            box_id: b.box_id(),
            value: *b.value.as_u64(),
            ergo_tree: base16::encode_lower(&b.ergo_tree.sigma_serialize_bytes().unwrap()),
            assets: b.tokens.map_or(vec![], |tokens| {
                tokens.into_iter().map(CborToken::from).collect()
            }),
            additional_registers: b.additional_registers,
            creation_height: b.creation_height,
            transaction_id: b.transaction_id,
            index: b.index,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CborBlockTransaction {
    id: TxId,
    inputs: Vec<CborErgoBox>,
    data_inputs: Option<Vec<CborDataInput>>,
    outputs: Vec<CborErgoBox>,
}

impl From<BlockTransaction> for CborBlockTransaction {
    fn from(tx: BlockTransaction) -> Self {
        Self {
            id: tx.id,
            inputs: tx.inputs.into_iter().map(CborErgoBox::from).collect(),
            data_inputs: tx
                .data_inputs
                .map(|di| di.into_iter().map(CborDataInput::from).collect()),
            outputs: tx.outputs.into_iter().map(CborErgoBox::from).collect(),
        }
    }
}
