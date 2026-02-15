//! Sorted Set (ZSET) command implementations.
//!
//! Supported commands: ZADD, ZRANGE, ZRANK, ZSCORE, ZREM

use crate::db::RocksDB;
use crate::server::protocol::RespValue;

/// Prefix for sorted set keys in RocksDB.
const ZSET_PREFIX: &str = "z:";
/// Suffix for sorted set count storage.
const ZSET_COUNT_SUFFIX: &str = ":count";
/// Prefix for sorted set member storage.
const ZSET_MEMBER_PREFIX: &str = "zm:";

fn get_key(key: &str) -> String {
    format!("{}{}", ZSET_PREFIX, key)
}

fn get_count_key(key: &str) -> String {
    format!("{}{}", get_key(key), ZSET_COUNT_SUFFIX)
}

fn get_score_key(key: &str, member: &str) -> String {
    format!("{}:{}:s", get_key(key), member)
}

fn get_member_key(key: &str, member: &str) -> String {
    format!("{}:{}{}", ZSET_MEMBER_PREFIX, key, member)
}

pub fn zadd(db: &RocksDB, args: &[RespValue]) -> RespValue {
    // ZADD key score member [score member ...]
    // args: [key, score, member, score, member, ...]
    // Minimum: key + 1 score/member pair = 3 args
    // After key, should have even number of args (score + member pairs)
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespValue::error("Wrong number of arguments for 'zadd'");
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
        let score = match &args[i] {
            RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => {
                v.parse::<f64>().unwrap_or(0.0)
            }
            RespValue::Integer(n) => *n as f64,
            _ => {
                i += 2;
                continue;
            }
        };

        let member = match &args[i + 1] {
            RespValue::BulkString(Some(m)) => m,
            _ => {
                i += 2;
                continue;
            }
        };

        let score_key = get_score_key(key, member);
        let member_key = get_member_key(key, member);

        // Check if member already exists
        let is_new = matches!(db.get(&score_key), Ok(None));

        if db.put(&score_key, &score.to_string()).is_ok() {
            if db.put(&member_key, &score.to_string()).is_ok() {
                if is_new {
                    added += 1;
                }
            }
        }

        i += 2;
    }

    let new_count = current_count + added;
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(added as i64)
}

pub fn zrange(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 3 {
        return RespValue::error("Wrong number of arguments for 'zrange'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let start: isize = match &args[1] {
        RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => v.parse().unwrap_or(0),
        RespValue::Integer(n) => *n as isize,
        _ => 0,
    };

    let stop: isize = match &args[2] {
        RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => v.parse().unwrap_or(-1),
        RespValue::Integer(n) => *n as isize,
        _ => -1,
    };

    let withscores = args.get(3).map(|arg| {
        match arg {
            RespValue::BulkString(Some(v)) | RespValue::SimpleString(v) => {
                v.to_uppercase() == "WITHSCORES"
            }
            _ => false,
        }
    }).unwrap_or(false);

    let count_key = get_count_key(key);
    let current_count: usize = match db.get(&count_key) {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(_) => 0,
    };

    if current_count == 0 {
        return RespValue::Array(vec![]);
    }

    // Normalize indices
    let start = if start < 0 { current_count as isize + start } else { start };
    let stop = if stop < 0 { current_count as isize + stop } else { stop };

    let start = start.max(0) as usize;
    let stop = (stop.min(current_count as isize - 1)) as usize;

    if start > stop {
        return RespValue::Array(vec![]);
    }

    // For simplicity, we get all members and sort by score
    // In a real implementation, you'd want a more efficient sorted structure
    let mut members_with_scores: Vec<(String, f64)> = Vec::new();

    let zm_prefix = get_member_key(key, "");
    match db.keys_with_prefix(zm_prefix.as_bytes()) {
        Ok(keys) => {
            for key_bytes in keys {
                let member = String::from_utf8_lossy(&key_bytes)
                    .trim_start_matches(&zm_prefix)
                    .to_string();

                if let Ok(Some(score_str)) = db.get(&get_score_key(key, &member)) {
                    if let Ok(score) = score_str.parse::<f64>() {
                        members_with_scores.push((member, score));
                    }
                }
            }
        }
        Err(e) => return RespValue::error(e.to_string()),
    }

    // Sort by score (ascending)
    members_with_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let start = start.min(members_with_scores.len());
    let stop = stop.min(members_with_scores.len() - 1);

    let mut results = Vec::new();
    for i in start..=stop {
        let (member, score): &(String, f64) = &members_with_scores[i];
        results.push(RespValue::BulkString(Some(member.clone())));
        if withscores {
            results.push(RespValue::BulkString(Some(score.to_string())));
        }
    }

    RespValue::Array(results)
}

pub fn zrank(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'zrank'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let member = match &args[1] {
        RespValue::BulkString(Some(m)) => m,
        _ => return RespValue::error("Invalid member"),
    };

    let score_key = get_score_key(key, member);
    match db.get(&score_key) {
        Ok(Some(_)) => {
            // Get all members and sort to find rank
            let zm_prefix = get_member_key(key, "");
            let mut members_with_scores: Vec<(String, f64)> = Vec::new();

            if let Ok(keys) = db.keys_with_prefix(zm_prefix.as_bytes()) {
                for key_bytes in keys {
                    let m = String::from_utf8_lossy(&key_bytes)
                        .trim_start_matches(&zm_prefix)
                        .to_string();

                    if let Ok(Some(score_str)) = db.get(&get_score_key(key, &m)) {
                        if let Ok(score) = score_str.parse::<f64>() {
                            members_with_scores.push((m, score));
                        }
                    }
                }
            }

            members_with_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            for (i, (m, _)) in members_with_scores.iter().enumerate() {
                if m == member {
                    return RespValue::Integer(i as i64);
                }
            }

            RespValue::Null
        }
        Ok(None) => RespValue::Null,
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn zscore(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'zscore'");
    }

    let key = match &args[0] {
        RespValue::BulkString(Some(k)) => k,
        _ => return RespValue::error("Invalid key"),
    };

    let member = match &args[1] {
        RespValue::BulkString(Some(m)) => m,
        _ => return RespValue::error("Invalid member"),
    };

    let score_key = get_score_key(key, member);
    match db.get(&score_key) {
        Ok(Some(v)) => RespValue::BulkString(Some(v)),
        Ok(None) => RespValue::Null,
        Err(e) => RespValue::error(e.to_string()),
    }
}

pub fn zrem(db: &RocksDB, args: &[RespValue]) -> RespValue {
    if args.len() < 2 {
        return RespValue::error("Wrong number of arguments for 'zrem'");
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

        let score_key = get_score_key(key, member);
        let member_key = get_member_key(key, member);

        if db.delete(&score_key).is_ok() {
            if db.delete(&member_key).is_ok() {
                removed += 1;
            }
        }
    }

    let new_count = current_count.saturating_sub(removed);
    let _ = db.put(&count_key, &new_count.to_string());

    RespValue::Integer(removed as i64)
}
