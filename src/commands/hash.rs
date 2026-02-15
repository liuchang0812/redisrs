//! Hash command implementations.
//!
//! Supported commands: HSET, HGET, HGETALL, HDEL, HLEN

use crate::db::RocksDB;
use crate::server::protocol::RespValue;

/// Prefix for hash keys in RocksDB.
const HASH_PREFIX: &str = "h:";
/// Suffix for hash count storage.
const HASH_COUNT_SUFFIX: &str = ":count";

fn get_key(key: &str) -> String {
    format!("{}{}", HASH_PREFIX, key)
}

fn get_field_key(key: &str, field: &str) -> String {
    format!("{}:{}:f", get_key(key), field)
}

fn get_count_key(key: &str) -> String {
    format!("{}{}", get_key(key), HASH_COUNT_SUFFIX)
}

pub fn hset(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 3 || args.len() % 2 != 1 {
        return RespValue::error("Wrong number of arguments for 'hset'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let count_key = get_count_key(key);
    let current_count: usize = match db.get(&count_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(_) => 0,
    };

    let mut added = 0;
    let mut i = 1;
    while i < args.len() - 1 {
        let field = match &args[i] {
            RespValue::BulkString(Some(f)) => f,
            _ => {
                i += 2;
                continue;
            }
        };

        let value = match &args[i + 1] {
            RespValue::BulkString(Some(v)) => v,
            _ => {
                i += 2;
                continue;
            }
        };

        let field_key = get_field_key(key, field);
        let is_new = matches!(db.get(&field_key), Ok(None));

        if db.put(&field_key, value).is_ok() {
            if is_new {
                added += 1;
            }
        }

        i += 2;
    }

    let new_count = current_count + added;
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(added as i64)
}

pub fn hget(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'hget'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let field = match &args[1] {
        RespValue::BulkString(Some(f)) => f,
        _ => return RespValue::error("Invalid field"),
    };

    let field_key = get_field_key(key, field);
    match db.get(&field_key) {
        Ok(Some(v)) => RespValue::BulkString(Some(v)),
        Ok(None) => RespValue::Null,
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn hgetall(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'hgetall'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let hash_prefix = get_key(key);
    let prefix_bytes = hash_prefix.as_bytes();

    let mut results = Vec::new();
    match db.keys_with_prefix(prefix_bytes) {
        Ok(keys) => {
            for key_bytes in keys {
                let key_str = String::from_utf8_lossy(&key_bytes);
                // Extract field from key "h:key:field:f"
                if let Some(rest) = key_str.strip_prefix(&format!("{}:", hash_prefix)) {
                    if let Some(field) = rest.strip_suffix(":f") {
                        let field_key = get_field_key(key, field);
                        if let Ok(Some(value)) = db.get(&field_key) {
                            results.push(RespValue::BulkString(Some(field.to_string())));
                            results.push(RespValue::BulkString(Some(value)));
                        }
                    }
                }
            }
        }
        Err(e) => return RespValue::error(e.to_string()),
    }

    RespValue::Array(results)
}

pub fn hdel(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'hdel'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let count_key = get_count_key(key);
    let current_count: usize = match db.get(&count_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(_) => 0,
    };

    let mut deleted = 0;
    for arg in args[1..].iter() {
        let field = match arg {
            RespValue::BulkString(Some(f)) => f,
            _ => continue,
        };

        let field_key = get_field_key(key, field);
        if db.delete(&field_key).is_ok() {
            deleted += 1;
        }
    }

    let new_count = current_count.saturating_sub(deleted);
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(deleted as i64)
}

pub fn hlen(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'hlen'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let count_key = get_count_key(key);
    match db.get(&count_key) {
        Ok(Some(v)) => RespValue::Integer(v.parse().unwrap_or(0)),
        Ok(None) => RespValue::Integer(0),
        Err(e) => RespValue::error(e.to_string()),
    }
}
