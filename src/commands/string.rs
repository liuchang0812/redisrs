//! String command implementations.
//!
//! Supported commands: GET, SET, DEL, MGET, MSET, INCR, DECR, EXISTS, STRLEN

use crate::db::RocksDB;
use crate::server::protocol::RespValue;

/// Prefix for string keys in RocksDB.
const STRING_PREFIX: &str = "s:";

/// Constructs the storage key for a string.
fn get_key(key: &str) -> String {
    format!("{}{}", STRING_PREFIX, key)
}

/// GET command - retrieves a string value by key.
pub fn get(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'get'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    match db.get(&get_key(key)) {
        Ok(Some(value)) => RespValue::BulkString(Some(value)),
        Ok(None) => RespValue::Null,
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn set(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'set'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let value = match &args[1] {
        RespValue::BulkString(Some(v)) => v,
        _ => return RespValue::error("Invalid value"),
    };

    match db.put(&get_key(key), value) {
        Ok(()) => RespValue::ok(),
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn del(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.is_empty() {
        return RespValue::error("Wrong number of arguments for 'del'");
    }

    let mut deleted = 0;
    for arg in args {
        let key = match arg {
            RespValue::BulkString(Some(k)) => k,
            _ => continue,
        };

        match db.delete(&get_key(key)) {
            Ok(()) => deleted += 1,
            Err(_) => {}
        }
    }

    RespValue::Integer(deleted)
}

pub fn mget(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.is_empty() {
        return RespValue::error("Wrong number of arguments for 'mget'");
    }

    let mut results = Vec::new();
    for arg in args {
        let key = match arg {
            RespValue::BulkString(Some(k)) => k,
            _ => {
                results.push(RespValue::Null);
                continue;
            }
        };

        match db.get(&get_key(key)) {
            Ok(Some(value)) => results.push(RespValue::BulkString(Some(value))),
            Ok(None) => results.push(RespValue::Null),
            Err(e) => return RespValue::error(e.to_string()),
        }
    }

    RespValue::Array(results)
}

pub fn mset(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 || args.len() % 2 != 0 {
        return RespValue::error("Wrong number of arguments for 'mset'");
    }

    for i in (0..args.len()).step_by(2) {
        let key = match &args[i] {
            RespValue::BulkString(Some(k)) => k,
            _ => return RespValue::error("Invalid key"),
        };

        let value = match &args[i + 1] {
            RespValue::BulkString(Some(v)) => v,
            _ => return RespValue::error("Invalid value"),
        };

        if let Err(e) = db.put(&get_key(key), value) {
            return RespValue::error(e.to_string());
        }
    }

    RespValue::ok()
}

pub fn incr(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'incr'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let db_key = get_key(key);
    let current: i64 = match db.get(&db_key) {
        Ok(Some(v)) => match v.parse() {
            Ok(n) => n,
            Err(_) => return RespValue::error("Value is not an integer"),
        },
        Ok(None) => 0,
        Err(e) => return RespValue::error(e.to_string()),
    };

    let new_value = current + 1;
    if let Err(e) = db.put(&db_key, &new_value.to_string()) {
        return RespValue::error(e.to_string());
    }

    RespValue::Integer(new_value)
}

pub fn decr(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'decr'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let db_key = get_key(key);
    let current: i64 = match db.get(&db_key) {
        Ok(Some(v)) => match v.parse() {
            Ok(n) => n,
            Err(_) => return RespValue::error("Value is not an integer"),
        },
        Ok(None) => 0,
        Err(e) => return RespValue::error(e.to_string()),
    };

    let new_value = current - 1;
    if let Err(e) = db.put(&db_key, &new_value.to_string()) {
        return RespValue::error(e.to_string());
    }

    RespValue::Integer(new_value)
}

pub fn exists(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.is_empty() {
        return RespValue::error("Wrong number of arguments for 'exists'");
    }

    let mut count = 0;
    for arg in args {
        let key = match arg {
            RespValue::BulkString(Some(k)) => k,
            _ => continue,
        };

        match db.get(&get_key(key)) {
            Ok(Some(_)) => count += 1,
            _ => {}
        }
    }

    RespValue::Integer(count)
}

pub fn strlen(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 1 {
        return RespValue::error("Wrong number of arguments for 'strlen'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    match db.get(&get_key(key)) {
        Ok(Some(value)) => RespValue::Integer(value.len() as i64),
        Ok(None) => RespValue::Integer(0),
        Err(e) => RespValue::error(e.to_string()),
    }
}
