use ergo_lib::chain::transaction::prover_result::ProverResult;
use ergo_lib::chain::transaction::{DataInput, Input, Transaction, TxId, TxIoVec};
use ergo_lib::ergo_chain_types::Header;
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::{ContextExtension, ProofBytes};
use ergo_lib::ergotree_ir::chain::ergo_box::{ErgoBox, ErgoBoxCandidate};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo {
    pub full_height: u32,
}

/// Transaction type for FullBlock where inputs are ErgoBox instead of Input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlockTransaction {
    /// transaction id
    pub id: TxId,
    /// inputs, that will be spent by this transaction (as ErgoBox instead of Input)
    pub inputs: TxIoVec<ErgoBox>,
    /// inputs, that are not going to be spent by transaction, but will be reachable from inputs
    /// scripts. `dataInputs` scripts will not be executed, thus their scripts costs are not
    /// included in transaction cost and they do not contain spending proofs.
    pub data_inputs: Option<TxIoVec<DataInput>>,
    /// Boxes to be created by this transaction. Differ from [`Self::output_candidates`] in that
    /// they include transaction id and index
    pub outputs: TxIoVec<ErgoBox>,
}

impl BlockTransaction {
    /// Convert BlockTransaction to standard Transaction by creating empty proofs for inputs
    pub fn to_transaction(
        self,
    ) -> Result<Transaction, ergo_lib::ergotree_ir::serialization::SigmaSerializationError> {
        let inputs = self.inputs.mapped_ref(|ergo_box| {
            Input::new(
                ergo_box.box_id(),
                ProverResult {
                    proof: ProofBytes::Empty,
                    extension: ContextExtension::empty(),
                },
            )
        });

        // Convert ErgoBox outputs to ErgoBoxCandidate for transaction creation
        let output_candidates = self.outputs.mapped_ref(|ergo_box| ErgoBoxCandidate {
            value: ergo_box.value,
            ergo_tree: ergo_box.ergo_tree.clone(),
            tokens: ergo_box.tokens.clone(),
            additional_registers: ergo_box.additional_registers.clone(),
            creation_height: ergo_box.creation_height,
        });

        Transaction::new(inputs, self.data_inputs, output_candidates)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBlock {
    pub header: Header,
    pub transactions: Vec<BlockTransaction>,
}
