//! Command implementations for all supported Redis data types.
//!
//! Modules:
//! - `string`: String commands (GET, SET, INCR, etc.)
//! - `list`: List commands (LPUSH, RPUSH, LRANGE, etc.)
//! - `set`: Set commands (SADD, SREM, SMEMBERS, etc.)
//! - `zset`: Sorted set commands (ZADD, ZRANGE, etc.)
//! - `hash`: Hash commands (HSET, HGET, HGETALL, etc.)

pub mod string;
pub mod list;
pub mod set;
pub mod zset;
pub mod hash;

use crate::db::RocksDB;
use crate::server::protocol::RespValue;
use std::sync::Arc;

/// Executes Redis commands against the RocksDB storage.
pub struct CommandExecutor {
    db: Arc<RocksDB>,
}

impl CommandExecutor {
    /// Creates a new CommandExecutor with the given database.
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    /// Dispatches a command to the appropriate handler.
    ///
    /// # Arguments
    /// * `command` - The Redis command name (e.g., "GET", "SET")
    /// * `args` - The command arguments (excluding the command name)
    ///
    /// # Returns
    /// A RespValue representing the command result.
    pub fn execute(&self, command: &str, args: &[RespValue]) -> RespValue {
        match command {
            // Basic commands
            "PING" => RespValue::simple_string("PONG"),
            "ECHO" => {
                if let Some(RespValue::BulkString(Some(s))) = args.first() {
                    RespValue::bulk_string(Some(s.clone()))
                } else {
                    RespValue::error("Invalid argument for ECHO")
                }
            }

            // String commands
            "GET" => string::get(self.db.as_ref(), args),
            "SET" => string::set(self.db.as_ref(), args),
            "DEL" => string::del(self.db.as_ref(), args),
            "MGET" => string::mget(self.db.as_ref(), args),
            "MSET" => string::mset(self.db.as_ref(), args),
            "INCR" => string::incr(self.db.as_ref(), args),
            "DECR" => string::decr(self.db.as_ref(), args),
            "EXISTS" => string::exists(self.db.as_ref(), args),
            "STRLEN" => string::strlen(self.db.as_ref(), args),

            // List commands
            "LPUSH" => list::lpush(self.db.as_ref(), args),
            "RPUSH" => list::rpush(self.db.as_ref(), args),
            "LPOP" => list::lpop(self.db.as_ref(), args),
            "RPOP" => list::rpop(self.db.as_ref(), args),
            "LRANGE" => list::lrange(self.db.as_ref(), args),
            "LLEN" => list::llen(self.db.as_ref(), args),

            // Set commands
            "SADD" => set::sadd(self.db.as_ref(), args),
            "SREM" => set::srem(self.db.as_ref(), args),
            "SMEMBERS" => set::smembers(self.db.as_ref(), args),
            "SISMEMBER" => set::sismember(self.db.as_ref(), args),
            "SCARD" => set::scard(self.db.as_ref(), args),

            // Sorted Set commands
            "ZADD" => zset::zadd(self.db.as_ref(), args),
            "ZRANGE" => zset::zrange(self.db.as_ref(), args),
            "ZRANK" => zset::zrank(self.db.as_ref(), args),
            "ZSCORE" => zset::zscore(self.db.as_ref(), args),
            "ZREM" => zset::zrem(self.db.as_ref(), args),

            // Hash commands
            "HSET" => hash::hset(self.db.as_ref(), args),
            "HGET" => hash::hget(self.db.as_ref(), args),
            "HGETALL" => hash::hgetall(self.db.as_ref(), args),
            "HDEL" => hash::hdel(self.db.as_ref(), args),
            "HLEN" => hash::hlen(self.db.as_ref(), args),

            _ => RespValue::error(format!("Unknown command: {}", command)),
        }
    }
}
