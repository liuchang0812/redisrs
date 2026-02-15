//! TCP connection handler for Redis client connections.
//!
//! Handles reading commands from clients, executing them, and sending responses.

use crate::db::RocksDB;
use crate::commands::CommandExecutor;
use super::protocol::{RespParser, RespValue};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Represents a client connection to the Redis server.
pub struct Connection {
    db: Arc<RocksDB>,
}

impl Connection {
    /// Creates a new Connection with the given database.
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    /// Handles a single client connection.
    ///
    /// Reads commands from the client, parses them using RESP protocol,
    /// executes the commands, and writes responses back to the client.
    pub async fn handle(self, mut stream: TcpStream) -> std::io::Result<()> {
        let mut parser = RespParser::new();
        let mut buf = [0u8; 1024];

        loop {
            let n = stream.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            parser.append(&buf[..n]);

            loop {
                match parser.parse() {
                    Ok(Some(value)) => {
                        parser.reset();
                        let response = self.process_command(value);
                        stream.write_all(response.to_resp().as_bytes()).await?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let response = RespValue::error(e.to_string());
                        stream.write_all(response.to_resp().as_bytes()).await?;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn process_command(&self, value: RespValue) -> RespValue {
        let RespValue::Array(args) = value else {
            return RespValue::error("Protocol error: expected array");
        };

        if args.is_empty() {
            return RespValue::error("Wrong number of arguments");
        }

        let command = match &args[0] {
            RespValue::BulkString(Some(s)) => s.to_uppercase(),
            RespValue::SimpleString(s) => s.to_uppercase(),
            _ => return RespValue::error("Invalid command"),
        };

        let cmd_exec = CommandExecutor::new(self.db.clone());
        cmd_exec.execute(&command, &args[1..])
    }
}
