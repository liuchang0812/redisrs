//! RESP (Redis Serialization Protocol) parser and serializer.
//!
//! Implements the Redis protocol for communication between client and server.
//! Reference: https://redis.io/docs/reference/protocol-spec/

use thiserror::Error;

/// Error types for protocol parsing.
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid protocol: {0}")]
    Invalid(String),
    #[error("Incomplete data")]
    Incomplete,
}

/// Result type for protocol operations.
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Represents values in the RESP format.
#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    /// Simple string: +OK\r\n
    SimpleString(String),
    /// Error: -ERR message\r\n
    Error(String),
    /// Integer: :1000\r\n
    Integer(i64),
    /// Bulk string: $6\r\nfoobar\r\n or null: $-1\r\n
    BulkString(Option<String>),
    /// Array: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n or null: *-1\r\n
    Array(Vec<RespValue>),
    /// Null: used for nil values in Redis
    Null,
}

impl RespValue {
    /// Converts the RespValue to its RESP wire format string.
    pub fn to_resp(&self) -> String {
        match self {
            RespValue::SimpleString(s) => format!("+{}\r\n", s),
            RespValue::Error(e) => format!("-ERR {}\r\n", e),
            RespValue::Integer(n) => format!(":{}\r\n", n),
            RespValue::BulkString(Some(s)) => format!("${}\r\n{}\r\n", s.len(), s),
            RespValue::BulkString(None) => "$-1\r\n".to_string(),
            RespValue::Array(arr) => {
                let mut result = format!("*{}\r\n", arr.len());
                for item in arr {
                    result.push_str(&item.to_resp());
                }
                result
            }
            RespValue::Null => "*-1\r\n".to_string(),
        }
    }

    /// Creates a SimpleString with value "OK".
    pub fn ok() -> Self {
        RespValue::SimpleString("OK".to_string())
    }

    /// Creates a SimpleString from a string.
    pub fn simple_string(s: impl Into<String>) -> Self {
        RespValue::SimpleString(s.into())
    }

    /// Creates a BulkString from an Option.
    pub fn bulk_string(s: Option<impl Into<String>>) -> Self {
        match s {
            Some(s) => RespValue::BulkString(Some(s.into())),
            None => RespValue::BulkString(None),
        }
    }

    /// Creates an Error response.
    pub fn error(s: impl Into<String>) -> Self {
        RespValue::Error(s.into())
    }
}

pub struct RespParser {
    pub buffer: Vec<u8>,
    pub position: usize,
}

impl RespParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
        }
    }

    pub fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn parse(&mut self) -> ProtocolResult<Option<RespValue>> {
        if self.position >= self.buffer.len() {
            return Ok(None);
        }

        match self.parse_value() {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn parse_value(&mut self) -> ProtocolResult<Option<RespValue>> {
        if self.position >= self.buffer.len() {
            return Ok(None);
        }

        let byte = self.buffer[self.position];
        match byte {
            b'+' => self.parse_simple_string(),
            b'-' => self.parse_error(),
            b':' => self.parse_integer(),
            b'$' => self.parse_bulk_string(),
            b'*' => self.parse_array(),
            _ => Err(ProtocolError::Invalid(format!(
                "Unknown type byte: {} ('{}')",
                byte, byte as char
            ))),
        }
    }

    fn parse_simple_string(&mut self) -> ProtocolResult<Option<RespValue>> {
        // Skip '+'
        self.position += 1;
        let end = self.find_crlf()?;
        let s = String::from_utf8(self.buffer[self.position..end].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;
        self.position = end + 2; // Skip \r\n
        Ok(Some(RespValue::SimpleString(s)))
    }

    fn parse_error(&mut self) -> ProtocolResult<Option<RespValue>> {
        // Skip '-'
        self.position += 1;
        let end = self.find_crlf()?;
        let s = String::from_utf8(self.buffer[self.position..end].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;
        self.position = end + 2; // Skip \r\n
        Ok(Some(RespValue::Error(s)))
    }

    fn parse_integer(&mut self) -> ProtocolResult<Option<RespValue>> {
        // Skip ':'
        self.position += 1;
        let end = self.find_crlf()?;
        let s = String::from_utf8(self.buffer[self.position..end].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;
        let n: i64 = s.parse()
            .map_err(|e| ProtocolError::Invalid(format!("Invalid integer: {}", e)))?;
        self.position = end + 2; // Skip \r\n
        Ok(Some(RespValue::Integer(n)))
    }

    fn parse_bulk_string(&mut self) -> ProtocolResult<Option<RespValue>> {
        // Skip '$'
        self.position += 1;

        // Find the length
        let len_end = self.find_crlf()?;
        let len_str = String::from_utf8(self.buffer[self.position..len_end].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;
        let len: isize = len_str.parse()
            .map_err(|e| ProtocolError::Invalid(format!("Invalid bulk string length: {}", e)))?;

        // Position now at start of data (after \r\n)
        let data_start = len_end + 2;

        // Null bulk string
        if len == -1 {
            self.position = data_start;
            return Ok(Some(RespValue::BulkString(None)));
        }

        let len = len as usize;

        // Check if we have enough data: need len bytes for data + 2 for trailing \r\n
        if data_start + len + 2 > self.buffer.len() {
            return Ok(None);
        }

        // Empty bulk string
        if len == 0 {
            self.position = data_start + 2; // Skip \r\n
            return Ok(Some(RespValue::BulkString(Some(String::new()))));
        }

        let data = String::from_utf8(self.buffer[data_start..data_start + len].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;

        // Skip data and trailing \r\n
        self.position = data_start + len + 2;

        Ok(Some(RespValue::BulkString(Some(data))))
    }

    fn parse_array(&mut self) -> ProtocolResult<Option<RespValue>> {
        // Skip '*'
        self.position += 1;
        let end = self.find_crlf()?;
        let count_str = String::from_utf8(self.buffer[self.position..end].to_vec())
            .map_err(|e| ProtocolError::Invalid(e.to_string()))?;
        let count: isize = count_str.parse()
            .map_err(|e| ProtocolError::Invalid(format!("Invalid array count: {}", e)))?;

        // Skip past count line
        self.position = end + 2;

        if count == -1 {
            return Ok(Some(RespValue::Null));
        }

        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            match self.parse_value()? {
                Some(value) => items.push(value),
                None => {
                    return Ok(None);
                }
            }
        }

        Ok(Some(RespValue::Array(items)))
    }

    fn find_crlf(&self) -> ProtocolResult<usize> {
        for i in self.position..self.buffer.len() - 1 {
            if self.buffer[i] == b'\r' && self.buffer[i + 1] == b'\n' {
                return Ok(i);
            }
        }
        Err(ProtocolError::Incomplete)
    }

    pub fn reset(&mut self) {
        // Remove parsed data from buffer
        if self.position > 0 {
            self.buffer.drain(0..self.position);
            self.position = 0;
        }
    }
}

impl Default for RespParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let mut parser = RespParser::new();
        parser.append(b"+OK\r\n");
        let result = parser.parse().unwrap().unwrap();
        assert_eq!(result, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_integer() {
        let mut parser = RespParser::new();
        parser.append(b":1000\r\n");
        let result = parser.parse().unwrap().unwrap();
        assert_eq!(result, RespValue::Integer(1000));
    }

    #[test]
    fn test_bulk_string() {
        let mut parser = RespParser::new();
        parser.append(b"$6\r\nfoobar\r\n");
        let result = parser.parse().unwrap().unwrap();
        assert_eq!(result, RespValue::BulkString(Some("foobar".to_string())));
    }

    #[test]
    fn test_array() {
        let mut parser = RespParser::new();
        parser.append(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
        let result = parser.parse().unwrap().unwrap();
        assert_eq!(result, RespValue::Array(vec![
            RespValue::BulkString(Some("foo".to_string())),
            RespValue::BulkString(Some("bar".to_string())),
        ]));
    }

    #[test]
    fn test_command() {
        let mut parser = RespParser::new();
        parser.append(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
        let result = parser.parse().unwrap().unwrap();
        assert_eq!(result, RespValue::Array(vec![
            RespValue::BulkString(Some("SET".to_string())),
            RespValue::BulkString(Some("foo".to_string())),
            RespValue::BulkString(Some("bar".to_string())),
        ]));
    }

    #[test]
    fn test_multiple_commands() {
        let mut parser = RespParser::new();
        // Two commands: SET and GET
        parser.append(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n");

        // Parse first command
        let result1 = parser.parse().unwrap().unwrap();
        assert_eq!(result1, RespValue::Array(vec![
            RespValue::BulkString(Some("SET".to_string())),
            RespValue::BulkString(Some("foo".to_string())),
            RespValue::BulkString(Some("bar".to_string())),
        ]));

        // Reset and parse second command
        parser.reset();
        let result2 = parser.parse().unwrap().unwrap();
        assert_eq!(result2, RespValue::Array(vec![
            RespValue::BulkString(Some("GET".to_string())),
            RespValue::BulkString(Some("foo".to_string())),
        ]));
    }

    #[test]
    fn test_lpush_format() {
        let mut parser = RespParser::new();
        // LPUSH mylist a b - note: $6 for mylist (6 chars)
        parser.append(b"*4\r\n$5\r\nLPUSH\r\n$6\r\nmylist\r\n$1\r\na\r\n$1\r\nb\r\n");

        let result = parser.parse().unwrap().unwrap();

        assert_eq!(result, RespValue::Array(vec![
            RespValue::BulkString(Some("LPUSH".to_string())),
            RespValue::BulkString(Some("mylist".to_string())),
            RespValue::BulkString(Some("a".to_string())),
            RespValue::BulkString(Some("b".to_string())),
        ]));
    }
}
