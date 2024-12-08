#!/bin/bash

kafka-topics --create --if-not-exists --bootstrap-server kafka1:9092 --topic blocks_topic --partitions 1 --replication-factor 1
kafka-topics --create --if-not-exists --bootstrap-server kafka1:9092 --topic tx_topic --partitions 1 --replication-factor 1
kafka-topics --create --if-not-exists --bootstrap-server kafka1:9092 --topic mempool_topic --partitions 1 --replication-factor 1

# List all topics
kafka-topics --bootstrap-server kafka1:9092 --list