//! Redis-compatible server implementation using RocksDB as storage.
//!
//! This is an educational implementation of a Redis-compatible server
//! that supports main data types: String, List, Set, Sorted Set, and Hash.
//! All data is persisted to RocksDB for durability.

mod db;
mod server;
mod commands;
mod utils;

use db::RocksDB;
use server::Connection;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task;

/// Main entry point for the Redis-compatible server.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Redis-compatible server on port 6379...");

    // Initialize RocksDB
    let db = match RocksDB::new("./redis_data") {
        Ok(db) => Arc::new(db),
        Err(e) => {
            eprintln!("Failed to initialize RocksDB: {}", e);
            std::process::exit(1);
        }
    };

    println!("RocksDB initialized successfully");

    // Bind to TCP port
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    println!("Listening on 127.0.0.1:6379");

    loop {
        // Accept incoming connections
        let (stream, addr) = listener.accept().await?;
        println!("Accepted connection from: {}", addr);

        let db = db.clone();

        // Spawn a task to handle the connection
        task::spawn(async move {
            let connection = Connection::new(db);
            if let Err(e) = connection.handle(stream).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}
