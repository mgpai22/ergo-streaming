use std::cmp::max;
use std::sync::{Arc, Once};
use std::time::Duration;

use async_stream::stream;
use futures::lock::Mutex;
use futures::Stream;
use futures_timer::Delay;
use log::{error, info, trace};
use pin_project::pin_project;

use crate::cache::chain_cache::ChainCache;
use crate::client::node::{ErgoNetwork, Error};
use crate::model::Block;

pub mod cache;
pub mod client;
pub mod constants;
pub mod model;
pub mod rocksdb;

#[derive(Debug, Clone)]
pub enum ChainUpgrade {
    RollForward(Block),
    RollBackward(Block),
}

#[derive(Debug, Clone)]
struct SyncState {
    next_height: u32,
}

impl SyncState {
    fn upgrade(&mut self) {
        self.next_height += 1;
    }

    fn downgrade(&mut self) {
        self.next_height -= 1;
    }
}

#[async_trait::async_trait(?Send)]
pub trait InitChainSync<TChainSync> {
    async fn init(
        self,
        starting_height: u32,
        tip_reached_signal: Option<&'static Once>,
    ) -> TChainSync;
}

pub struct ChainSyncNonInit<'a, TClient, TCache> {
    client: &'a TClient,
    cache: TCache,
    batch_size: u32,
    chunk_size: usize,
    throttle_ms: u64,
}

impl<'a, TClient, TCache> ChainSyncNonInit<'a, TClient, TCache> {
    pub fn new(
        client: &'a TClient,
        cache: TCache,
        batch_size: u32,
        chunk_size: usize,
        throttle_ms: u64,
    ) -> Self {
        Self {
            client,
            cache,
            batch_size,
            chunk_size,
            throttle_ms,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<'a, TClient, TCache> InitChainSync<ChainSync<'a, TClient, TCache>>
    for ChainSyncNonInit<'a, TClient, TCache>
where
    TClient: ErgoNetwork + Send + Sync,
    TCache: ChainCache,
{
    async fn init(
        self,
        starting_height: u32,
        tip_reached_signal: Option<&'a Once>,
    ) -> ChainSync<TClient, TCache> {
        ChainSync::init(
            starting_height,
            self.client,
            self.cache,
            tip_reached_signal,
            self.batch_size,
            self.chunk_size,
            self.throttle_ms,
        )
        .await
    }
}

#[pin_project]
pub struct ChainSync<'a, TClient, TCache> {
    starting_height: u32,
    client: &'a TClient,
    cache: Arc<Mutex<TCache>>,
    state: Arc<Mutex<SyncState>>,
    #[pin]
    delay: Mutex<Option<Delay>>,
    tip_reached_signal: Option<&'a Once>,
    batch_size: u32,
    chunk_size: usize,
    throttle_ms: u64,
}

impl<'a, TClient, TCache> ChainSync<'a, TClient, TCache>
where
    TClient: ErgoNetwork + Send + Sync,
    TCache: ChainCache,
{
    pub async fn init(
        starting_height: u32,
        client: &'a TClient,
        mut cache: TCache,
        tip_reached_signal: Option<&'a Once>,
        batch_size: u32,
        chunk_size: usize,
        throttle_ms: u64,
    ) -> ChainSync<'a, TClient, TCache> {
        let best_block = cache.get_best_block().await;
        let start_at = if let Some(best_block) = best_block {
            trace!(target: "chain_sync", "Best block is [{}], height: {}", best_block.id, best_block.height);
            max(best_block.height, starting_height)
        } else {
            starting_height
        };
        Self {
            starting_height,
            client,
            cache: Arc::new(Mutex::new(cache)),
            state: Arc::new(Mutex::new(SyncState {
                next_height: start_at,
            })),
            delay: Mutex::new(None),
            tip_reached_signal,
            batch_size,
            chunk_size,
            throttle_ms,
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    /// Try acquiring next batch of upgrades from the network.
    /// `None` is returned when no upgrades are available at the moment.
    pub async fn try_upgrade(&self) -> Option<Vec<ChainUpgrade>> {
        let next_height = { self.state.lock().await.next_height };
        trace!(target: "chain_sync", "Processing height batch starting at [{}]", next_height);

        // Check best height before requesting to avoid going beyond chain tip.
        let best_height = match self.client.get_best_height().await {
            Ok(h) => h,
            Err(e) => {
                error!(target: "chain_sync", "Error getting best height: {:?}", e);
                return None;
            }
        };

        if next_height > best_height {
            trace!(target: "chain_sync", "next_height [{}] > best_height [{}], no new blocks available", next_height, best_height);
            return None;
        }

        match self
            .client
            .get_blocks_batch(next_height, self.batch_size, self.chunk_size)
            .await
        {
            Ok(api_blocks) => {
                trace!(target: "chain_sync", "Got {} blocks from API", api_blocks.len());
                let mut upgrades = Vec::new();
                for api_blk in api_blocks {
                    let block_height = api_blk.header.height;
                    // If the returned block height is less than the requested next_height,
                    // it means we're getting a top block because we've exceeded the chain tip.
                    // Treat this as no new block scenario and break.
                    if block_height < next_height {
                        trace!(target: "chain_sync", "Received block at height [{}] which is less than requested next_height [{}]. No new block scenario.", block_height, next_height);
                        break;
                    }

                    let mut cache = self.cache.lock().await;

                    // Check if we already have this block
                    if cache.exists(api_blk.header.id).await {
                        trace!(
                            target: "chain_sync",
                            "Skipping block [{}], already in cache at height: {}",
                            api_blk.header.id,
                            block_height
                        );
                        self.state.lock().await.upgrade();
                        continue;
                    }

                    let parent_id = api_blk.header.parent_id;
                    let linked = cache.exists(parent_id).await;
                    if linked || block_height == self.starting_height {
                        trace!(target: "chain_sync", "Chain is linked, upgrading ..");
                        let blk = Block::from(api_blk);
                        cache.append_block(blk.clone()).await;
                        self.state.lock().await.upgrade();
                        upgrades.push(ChainUpgrade::RollForward(blk));
                    } else {
                        // Local chain does not link anymore
                        trace!(target: "chain_sync", "Chain does not link, downgrading ..");
                        if let Some(discarded_blk) = cache.take_best_block().await {
                            self.state.lock().await.downgrade();
                            upgrades.push(ChainUpgrade::RollBackward(discarded_blk));
                            // Stop processing batch after rollback
                            break;
                        }
                    }
                }
                if !upgrades.is_empty() {
                    Some(upgrades)
                } else {
                    None
                }
            }
            Err(e) => {
                println!("try_upgrade error details: {:?}", e);
                error!(target: "chain_sync", "try_upgrade error details: {:?}", e);

                match e {
                    Error::NoBlock => {
                        trace!(target: "chain_sync", "No blocks found at height {}", next_height);
                    }
                    Error::Json(err) => {
                        error!(target: "chain_sync", "JSON decoding error: {}", err);
                    }
                    _ => {
                        error!(target: "chain_sync", "Unexpected error: {}", e);
                    }
                }
                None
            }
        }
    }
}

pub fn chain_sync_stream<'a, TClient, TCache>(
    chain_sync: ChainSync<'a, TClient, TCache>,
) -> impl Stream<Item = ChainUpgrade> + 'a
where
    TClient: ErgoNetwork + Send + Sync + Unpin,
    TCache: ChainCache + Unpin + 'a,
{
    stream! {
        loop {
            let delay = {chain_sync.delay.lock().await.take()};
            if let Some(delay) = delay {
                delay.await;
            }
            if let Some(upgrades) = chain_sync.try_upgrade().await {
                for upgrade in upgrades {
                    yield upgrade;
                }
            } else {
                *chain_sync.delay.lock().await = Some(Delay::new(Duration::from_millis(chain_sync.throttle_ms)));
                if let Some(sig) = chain_sync.tip_reached_signal {
                    sig.call_once(|| {
                        trace!(target: "chain_sync", "Tip reached, waiting for new blocks ..");
                    });
                }
            }
        }
    }
}
