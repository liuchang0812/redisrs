//! RocksDB database wrapper for persistent storage.
//!
//! Provides a simple interface to RocksDB with UTF-8 string handling.

use rocksdb::{DB, Options};
use std::path::Path;
use thiserror::Error;

/// Error types for database operations.
#[derive(Error, Debug)]
pub enum DbError {
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Result type for database operations.
pub type DbResult<T> = Result<T, DbError>;

/// Wrapper around RocksDB for key-value storage.
pub struct RocksDB {
    db: DB,
}

impl RocksDB {
    /// Opens or creates a RocksDB database at the given path.
    pub fn new(path: impl AsRef<Path>) -> DbResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }

    /// Retrieves a value by key, returns None if key doesn't exist.
    pub fn get(&self, key: impl AsRef<[u8]>) -> DbResult<Option<String>> {
        match self.db.get(key.as_ref())? {
            Some(value) => Ok(Some(String::from_utf8(value.to_vec())?)),
            None => Ok(None),
        }
    }

    /// Stores a key-value pair in the database.
    pub fn put(&self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> DbResult<()> {
        self.db.put(key.as_ref(), value.as_ref())?;
        Ok(())
    }

    /// Deletes a key from the database.
    pub fn delete(&self, key: impl AsRef<[u8]>) -> DbResult<()> {
        self.db.delete(key.as_ref())?;
        Ok(())
    }

    /// Returns all keys that start with the given prefix.
    pub fn keys_with_prefix(&self, prefix: &[u8]) -> DbResult<Vec<Vec<u8>>> {
        let mut keys = Vec::new();
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, _) = item?;
            keys.push(key.to_vec());
        }
        Ok(keys)
    }

}
