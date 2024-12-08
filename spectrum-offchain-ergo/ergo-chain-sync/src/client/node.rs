use async_trait::async_trait;
use derive_more::From;
use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergo_chain_types::{BlockId, Header};
use isahc::{AsyncReadResponseExt, HttpClient};
use log::{error, info};
use thiserror::Error;

use crate::client::model::{ApiInfo, FullBlock};
use crate::client::types::Url;

use super::types::with_path;

#[derive(Error, From, Debug)]
pub enum Error {
    #[error("json decoding: {0}")]
    Json(serde_json::Error),
    #[error("isahc: {0}")]
    Isahc(isahc::Error),
    #[error("http: {0}")]
    Http(isahc::http::Error),
    #[error("io: {0}")]
    Io(std::io::Error),
    #[error("unsuccessful request: {0}")]
    UnsuccessfulRequest(String),
    #[error("No block found")]
    NoBlock,
}

#[async_trait]
pub trait ErgoNetwork: Send + Sync {
    async fn get_blocks_range(
        &self,
        from_height: u32,
        to_height: u32,
    ) -> Result<Vec<BlockId>, Error>;
    async fn get_best_height(&self) -> Result<u32, Error>;
    async fn fetch_mempool(&self, offset: usize, limit: usize) -> Result<Vec<Transaction>, Error>;

    async fn get_blocks_batch(
        &self,
        from_height: u32,
        batch_size: u32,
        chunk_size: usize,
    ) -> Result<Vec<FullBlock>, Error> {
        let batch_end = from_height + batch_size;
        info!(target: "chain_sync", "Fetching blocks range from {} to {}", from_height, batch_end);
        let block_ids = self.get_blocks_range(from_height, batch_end).await?;
        info!(target: "chain_sync", "Got {} blocks from API", block_ids.len());
        if block_ids.is_empty() {
            return Err(Error::NoBlock);
        }
        self.get_full_blocks_in_chunks(block_ids, chunk_size).await
    }

    async fn get_full_blocks_in_chunks(
        &self,
        block_ids: Vec<BlockId>,
        chunk_size: usize,
    ) -> Result<Vec<FullBlock>, Error> {
        let mut full_blocks = Vec::with_capacity(block_ids.len());

        // Process in smaller chunks to avoid large payload errors
        for chunk in block_ids.chunks(chunk_size) {
            let chunk_result = self.get_full_blocks(chunk.to_vec()).await?;
            full_blocks.extend(chunk_result);
        }

        Ok(full_blocks)
    }

    async fn get_full_blocks(&self, block_ids: Vec<BlockId>) -> Result<Vec<FullBlock>, Error>;
}

#[derive(Clone)]
pub struct ErgoNodeHttpClient {
    pub client: HttpClient,
    pub base_url: Url,
}

impl ErgoNodeHttpClient {
    pub fn new(client: HttpClient, base_url: Url) -> Self {
        Self { client, base_url }
    }
}

#[async_trait]
impl ErgoNetwork for ErgoNodeHttpClient {
    async fn get_blocks_range(
        &self,
        from_height: u32,
        to_height: u32,
    ) -> Result<Vec<BlockId>, Error> {
        info!(target: "ergo_network", "Fetching blocks range from {} to {}", from_height, to_height);
        let mut resp = self
            .client
            .get_async(with_path(
                &self.base_url,
                &format!(
                    "/blocks/chainSlice?fromHeight={}&toHeight={}",
                    from_height - 1,
                    to_height + 1
                ),
            ))
            .await?;

        if resp.status().is_success() {
            let body = resp.text().await?;
            let headers: Vec<Header> = serde_json::from_str(&body)?;
            Ok(headers.into_iter().map(|h| h.id).collect())
        } else {
            Err(Error::UnsuccessfulRequest(format!(
                "expected 200 from /blocks/chainSlice, got {}",
                resp.status()
            )))
        }
    }

    async fn get_full_blocks(&self, block_ids: Vec<BlockId>) -> Result<Vec<FullBlock>, Error> {
        if block_ids.is_empty() {
            return Ok(vec![]);
        }

        let block_id_strings: Vec<String> = block_ids.iter().map(|id| id.to_string()).collect();
        let body_string = serde_json::to_string(&block_id_strings)?;

        info!(
            target: "ergo_network",
            "Requesting {} blocks from /blocks/headerIds",
            block_ids.len()
        );

        let request = isahc::Request::builder()
            .method("POST")
            .uri(with_path(&self.base_url, "/blocks/headerIds"))
            .header("Content-Type", "application/json")
            .header("accept", "application/json")
            .body(body_string.clone())?;

        let mut resp = self.client.send_async(request).await?;

        if resp.status().is_success() {
            resp.json().await.map_err(Error::from)
        } else {
            let error_body = resp.text().await?;
            error!("Unexpected response from node: {}", error_body);
            Err(Error::UnsuccessfulRequest(format!(
                "expected 200 from /blocks/headerIds, got {} with body: {}",
                resp.status(),
                error_body
            )))
        }
    }

    async fn get_best_height(&self) -> Result<u32, Error> {
        let mut resp = self
            .client
            .get_async(with_path(&self.base_url, "/info"))
            .await?;
        if resp.status().is_success() {
            let info: ApiInfo = resp.json().await?;
            Ok(info.full_height)
        } else {
            Err(Error::UnsuccessfulRequest(format!(
                "expected 200 from /info, got {}",
                resp.status()
            )))
        }
    }

    async fn fetch_mempool(&self, offset: usize, limit: usize) -> Result<Vec<Transaction>, Error> {
        let mut resp = self
            .client
            .get_async(with_path(
                &self.base_url,
                &format!(
                    "/transactions/unconfirmed?offset={}&limit={}",
                    offset, limit
                ),
            ))
            .await?;

        if resp.status().is_success() {
            resp.json().await.map_err(Error::from)
        } else {
            Err(Error::UnsuccessfulRequest(format!(
                "expected 200 from /transactions/unconfirmed, got {}",
                resp.status()
            )))
        }
    }
}
