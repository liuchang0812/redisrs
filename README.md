# Redis-Compatible Server

An educational implementation of a Redis-compatible server built on RocksDB.

## Features

- **TCP Server**: Listens on port 6379, compatible with `redis-cli`
- **RESP Protocol**: Full Redis Serialization Protocol parser
- **Data Types**:
  - String: `GET`, `SET`, `DEL`, `MGET`, `MSET`, `INCR`, `DECR`, `EXISTS`, `STRLEN`
  - List: `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`, `LLEN`
  - Set: `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`, `SCARD`
  - ZSet: `ZADD`, `ZRANGE`, `ZRANK`, `ZSCORE`, `ZREM`
  - Hash: `HSET`, `HGET`, `HGETALL`, `HDEL`, `HLEN`
- **Storage**: RocksDB persistence in `./redis_data` directory

## Quick Start

```bash
# Build
cargo build --release

# Run
cargo run --release

# Connect with redis-cli
redis-cli -p 6379

# Test commands
PING
SET foo bar
GET foo
LPUSH mylist a b c
LRANGE mylist 0 -1
```

## Testing

```bash
# Run unit tests
cargo test

# Run with specific test
cargo test test_bulk_string
```

## Project Structure

```
redisrs/
├── src/
│   ├── main.rs              # Entry point, TCP server
│   ├── server/
│   │   ├── connection.rs    # Connection handler
│   │   └── protocol.rs      # RESP protocol parser
│   ├── commands/
│   │   ├── string.rs        # String commands
│   │   ├── list.rs          # List commands
│   │   ├── set.rs           # Set commands
│   │   ├── zset.rs          # Sorted set commands
│   │   └── hash.rs          # Hash commands
│   └── db/
│       └── rocksdb.rs       # RocksDB wrapper
├── Cargo.toml
└── README.md
```

## Key Design

- **Custom KV Encoding**: Each data element is stored as a separate RocksDB key
- **Key Prefixes**:
  - String: `s:` + key
  - List: `l:` + key
  - Set: `set:` + key
  - ZSet: `z:` + key
  - Hash: `h:` + key

This is an educational project demonstrating storage engine and network protocol concepts.
