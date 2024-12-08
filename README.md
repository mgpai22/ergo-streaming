# Ergo Streaming

Ergo Streaming is a Kafka-based streaming service for Ergo chain data.

This service polls the Ergo node and streams blockchain data via Kafka, handling both chain progression and rollbacks.

There are three topics:
- `blocks_topic`: Block-related events
- `tx_topic`: Transaction-related events
- `mempool_topic`: Mempool transaction events

## Block Events
The `blocks_topic` fires when blocks are added or removed from the chain. The message is a JSON object with one of two types:

**BlockApply** (new block added):
```json
{
"timestamp": <block_timestamp>,
"height": <block_height>,
"id": <block_id>
}
```

**BlockUnapply** (block removed during rollback):
```json
{
"timestamp": <block_timestamp>,
"height": <block_height>,
"id": <block_id>
}
```


## Transaction Events
The `tx_topic` fires for transaction events. Messages can be either:

**AppliedEvent** (new transaction):
```json
{
"timestamp": <block_timestamp>,
"height": <block_height>,
"tx": <base64_encoded_transaction>
}
```


**UnappliedEvent** (transaction removed during rollback):
```json
{
"tx": <base64_encoded_transaction>
}
```


## Mempool Events
The `mempool_topic` tracks transactions entering and leaving the mempool:

**TxAccepted** (transaction entered mempool):
```json
{
"tx": <base64_encoded_transaction>
}
```

**TxWithdrawn** (transaction left mempool):
```json
{
"tx": <base64_encoded_transaction>
}
```

# Running
```
docker compose up --build -d
```


## Rollback Handling
The service handles blockchain reorganizations (rollbacks) by:
1. Emitting `BlockUnapply` events for each block being rolled back
2. Emitting `UnappliedEvent` for each transaction in those blocks (in reverse order)
3. Then emitting new `BlockApply` and `AppliedEvent` messages for the new chain

## Transaction Sequencing
All transaction events are guaranteed to be sequential and properly ordered relative to their blocks:

1. When a new block is added, you'll first receive a `BlockApply` event
2. This is followed by `AppliedEvent` messages for each transaction in that block
3. During rollbacks, you'll first receive a `BlockUnapply` event
4. This is followed by `UnappliedEvent` messages for each transaction in reverse order

This sequencing is enforced by the service's architecture:
- Block events are processed through `block_event_source`
- Transaction events are derived from these blocks via `tx_event_source`
- The `process_upgrade` function ensures transactions are handled in the correct order (forward for applies, reverse for rollbacks)

## Configuration Parameters

### Chain Sync Settings
- `chain_sync_starting_height`: The block height where chain synchronization begins (e.g., 1400000)
- `chain_sync_batch_size`: Number of blocks to request in a single batch from the node (e.g., 50). The larger, the faster the sync. However it puts too much strain on the node.
- `chain_sync_chunk_size`: Number of full blocks to retrive at once from node (e.g., 5). The larger, the faster the sync. However it puts too much strain on the node.

### Cache Settings
- `chain_cache_db_path`: Location for the RocksDB database storing chain state
- `mempool_cache_db_path`: Location for the RocksDB database storing mempool state

### Timing Parameters
- `http_client_timeout_duration_secs`: Maximum time to wait for node API responses (in seconds)
- `mempool_sync_interval`: How often to poll the mempool for changes (in seconds)

### Network Settings
- `node_addr`: Ergo node API endpoint
- `kafka_address`: Kafka broker address (format: "host:port")
- Topic names can be configured via `blocks_topic`, `tx_topic`, and `mempool_topic`


# Attribution

This project was forked from [Spectrum](https://github.com/spectrum-finance/ergo-analytics-events-streaming).

Its modified to sync a lot faster by using batch endpoints. The kafka service is also modified to use kraft rather than zookeeper.


