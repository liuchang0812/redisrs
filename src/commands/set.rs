//! Set command implementations.
//!
//! Supported commands: SADD, SREM, SMEMBERS, SISMEMBER, SCARD

use crate::db::RocksDB;
use crate::server::protocol::RespValue;

/// Prefix for set keys in RocksDB.
const SET_PREFIX: &str = "set:";
/// Suffix for set count storage.
const SET_COUNT_SUFFIX: &str = ":count";

fn get_key(key: &str) -> String {
    format!("{}{}", SET_PREFIX, key)
}

fn get_member_key(key: &str, member: &str) -> String {
    format!("{}:{}:m", get_key(key), member)
}

fn get_count_key(key: &str) -> String {
    format!("{}{}", get_key(key), SET_COUNT_SUFFIX)
}

pub fn sadd(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'sadd'");
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
    for arg in args[1..].iter() {
        let member = match arg {
            RespValue::BulkString(Some(m)) => m,
            _ => continue,
        };

        let member_key = get_member_key(key, member);
        // Check if member already exists
        match db.get(&member_key) {
            Ok(Some(_)) => continue,
            _ => {}
        }

        if db.put(&member_key, "1").is_ok() {
            added += 1;
        }
    }

    let new_count = current_count + added;
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(added as i64)
}

pub fn srem(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'srem'");
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

    let mut removed = 0;
    for arg in args[1..].iter() {
        let member = match arg {
            RespValue::BulkString(Some(m)) => m,
            _ => continue,
        };

        let member_key = get_member_key(key, member);
        if db.delete(&member_key).is_ok() {
            removed += 1;
        }
    }

    let new_count = current_count.saturating_sub(removed);
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(removed as i64)
}

pub fn smembers(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'smembers'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let set_prefix = get_key(key);
    let prefix_bytes = set_prefix.as_bytes();

    let mut members = Vec::new();
    match db.keys_with_prefix(prefix_bytes) {
        Ok(keys) => {
            for key_bytes in keys {
                // Extract member from key "set:key:member:m"
                let key_str = String::from_utf8_lossy(&key_bytes);
                if let Some(member) = key_str.strip_prefix(&format!("{}:", set_prefix)) {
                    if let Some(m) = member.strip_suffix(":m") {
                        members.push(RespValue::BulkString(Some(m.to_string())));
                    }
                }
            }
        }
        Err(e) => return RespValue::error(e.to_string()),
    }

    RespValue::Array(members)
}

pub fn sismember(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'sismember'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let member = match &args[1] {
        RespValue::BulkString(Some(m)) => m,
        _ => return RespValue::error("Invalid member"),
    };

    let member_key = get_member_key(key, member);
    match db.get(&member_key) {
        Ok(Some(_)) => RespValue::Integer(1),
        Ok(None) => RespValue::Integer(0),
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn scard(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'scard'");
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
