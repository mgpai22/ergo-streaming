use cache::chain_cache::{ChainCache, InMemoryCache};
use ergo_chain_sync::client::node::ErgoNodeHttpClient;
use ergo_chain_sync::*;

#[tokio::test]
async fn test_chain_sync_batch_processing() {
    // Create HTTP client and URL
    let http_client = isahc::HttpClient::new().unwrap();
    let base_url = "https://node.sigmaspace.io".parse().unwrap();
    let client = ErgoNodeHttpClient::new(http_client, base_url);

    // Create cache
    let cache = Box::new(InMemoryCache::new()) as Box<dyn ChainCache>;

    // Initialize chain sync from a recent height
    let chain_sync = ChainSyncNonInit::new(&client, cache).init(1000, None).await;

    // Test batch processing
    let upgrades = chain_sync.try_upgrade().await;
    assert!(upgrades.is_some());

    let upgrades = upgrades.unwrap();
    assert!(!upgrades.is_empty()); // Check that we got some blocks

    // Verify the blocks are in sequence
    for (i, upgrade) in upgrades.iter().enumerate() {
        if let ChainUpgrade::RollForward(block) = upgrade {
            assert_eq!(block.height, 1000 + i as u32);
            if i > 0 {
                // Verify block links to previous block
                if let ChainUpgrade::RollForward(prev_block) = &upgrades[i - 1] {
                    assert_eq!(block.parent_id, prev_block.id);
                }
            }
        }
    }
}
