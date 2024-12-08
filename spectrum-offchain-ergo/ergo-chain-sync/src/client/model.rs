use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergo_chain_types::Header;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockTransactions {
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo {
    pub full_height: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBlock {
    pub header: Header,
    #[serde(rename = "blockTransactions")]
    pub block_transactions: BlockTransactions,
    pub extension: Extension,
    pub ad_proofs: Option<AdProofs>,
    pub size: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub header_id: String,
    pub digest: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdProofs {
    // Add fields as needed
}
