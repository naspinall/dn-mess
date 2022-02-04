use core::fmt;

#[derive(Debug)]
pub enum NetworkBufferError {
    BufferFullError,
    BufferEmptyError,
    InvalidPacket,
    CompressionError,
}

impl std::error::Error for NetworkBufferError {}

impl fmt::Display for NetworkBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkBufferError::BufferEmptyError => write!(f, "Buffer Empty"),
            NetworkBufferError::BufferFullError => write!(f, "Buffer Full"),
            NetworkBufferError::InvalidPacket => write!(f, "Invalid Packet"),
            NetworkBufferError::CompressionError => write!(f, "Compression Error"),
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {}

impl std::error::Error for ConnectionError {}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _ => write!(f, "Connection error"),
        }
    }
}
