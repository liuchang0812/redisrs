//! List command implementations.
//!
//! Supported commands: LPUSH, RPUSH, LPOP, RPOP, LRANGE, LLEN

use crate::db::RocksDB;
use crate::server::protocol::RespValue;

/// Prefix for list keys in RocksDB.
const LIST_PREFIX: &str = "l:";

fn get_key(key: &str) -> String {
    format!("{}{}", LIST_PREFIX, key)
}

fn get_list_len_key(key: &str) -> String {
    format!("{}:len", get_key(key))
}

fn get_list_element_key(key: &str, index: usize) -> String {
    format!("{}:{}", get_key(key), index)
}

pub fn lpush(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'lpush'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let len_key = get_list_len_key(key);
    let current_len: usize = match db.get(&len_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(e) => return RespValue::error(e.to_string()),
    };

    // Shift existing elements to the right
    for i in (0..current_len).rev() {
        if let Ok(Some(value)) = db.get(&get_list_element_key(key, i)) {
            let _ = db.put(&get_list_element_key(key, i + 1), &value);
        }
    }

    // Insert new elements at the beginning
    let mut count = 0;
    for (i, arg) in args[1..].iter().enumerate() {
        let value = match arg {
            RespValue::BulkString(Some(v)) => v,
            _ => continue,
        };
        if db.put(&get_list_element_key(key, i), value).is_ok() {
            count += 1;
        }
    }

    let new_len = current_len + count;
    let _ = db.put(&len_key, &new_len.to_string());

    RespValue::Integer(new_len as i64)
}

pub fn rpush(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'rpush'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let len_key = get_list_len_key(key);
    let current_len: usize = match db.get(&len_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(e) => return RespValue::error(e.to_string()),
    };

    // Insert new elements at the end
    let mut count = 0;
    for (i, arg) in args[1..].iter().enumerate() {
        let value = match arg {
            RespValue::BulkString(Some(v)) => v,
            _ => continue,
        };
        if db.put(&get_list_element_key(key, current_len + i), value).is_ok() {
            count += 1;
        }
    }

    let new_len = current_len + count;
    let _ = db.put(&len_key, &new_len.to_string());

    RespValue::Integer(new_len as i64)
}

pub fn lpop(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'lpop'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let len_key = get_list_len_key(key);
    let current_len: usize = match db.get(&len_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => return RespValue::Null,
        Err(e) => return RespValue::error(e.to_string()),
    };

    if current_len == 0 {
        return RespValue::Null;
    }

    // Get first element
    let element_key = get_list_element_key(key, 0);
    let result = match db.get(&element_key) {
        Ok(Some(v)) => RespValue::BulkString(Some(v)),
        _ => RespValue::Null,
    };

    // Shift elements to the left
    for i in 1..current_len {
        if let Ok(Some(value)) = db.get(&get_list_element_key(key, i)) {
            let _ = db.put(&get_list_element_key(key, i - 1), &value);
        }
    }

    // Delete the last element
    let _ = db.delete(&get_list_element_key(key, current_len - 1));

    // Update length
    let _ = db.put(&len_key, &(current_len - 1).to_string());

    result
}

pub fn rpop(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'rpop'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let len_key = get_list_len_key(key);
    let current_len: usize = match db.get(&len_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => return RespValue::Null,
        Err(e) => return RespValue::error(e.to_string()),
    };

    if current_len == 0 {
        return RespValue::Null;
    }

    // Get last element
    let element_key = get_list_element_key(key, current_len - 1);
    let result = match db.get(&element_key) {
        Ok(Some(v)) => RespValue::BulkString(Some(v)),
        _ => RespValue::Null,
    };

    // Delete the last element
    let _ = db.delete(&element_key);

    // Update length
    let _ = db.put(&len_key, &(current_len - 1).to_string());

    result
}

pub fn lrange(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 3 {
        return RespValue::error("Wrong number of arguments for 'lrange'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let start: isize = match &args[1] {
        RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => {
            v.parse().unwrap_or(0)
        }
        RespValue::Integer(n) => *n as isize,
        _ => 0,
    };

    let stop: isize = match &args[2] {
        RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => {
            v.parse().unwrap_or(0)
        }
        RespValue::Integer(n) => *n as isize,
        _ => -1,
    };

    let len_key = get_list_len_key(key);
    let current_len: usize = match db.get(&len_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(_) => 0,
    };

    if current_len == 0 {
        return RespValue::Array(vec![]);
    }

    // Normalize indices
    let start = if start < 0 { current_len as isize + start } else { start };
    let stop = if stop < 0 { current_len as isize + stop } else { stop };

    let start = start.max(0) as usize;
    let stop = (stop.min(current_len as isize - 1)) as usize;

    if start > stop {
        return RespValue::Array(vec![]);
    }

    let mut results = Vec::new();
    for i in start..=stop {
        match db.get(&get_list_element_key(key, i)) {
            Ok(Some(value)) => results.push(RespValue::BulkString(Some(value))),
            _ => results.push(RespValue::Null),
        }
    }

    RespValue::Array(results)
}

pub fn llen(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'llen'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let len_key = get_list_len_key(key);
    match db.get(&len_key) {
        Ok(Some(v)) => RespValue::Integer(v.parse().unwrap_or(0)),
        Ok(None) => RespValue::Integer(0),
        Err(e) => RespValue::error(e.to_string()),
    }
}
