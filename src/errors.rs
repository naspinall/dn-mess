use core::fmt;

#[derive(Debug)]
pub enum NetworkBufferError {
    BufferFullError,
    BufferEmptyError,
}

impl std::error::Error for NetworkBufferError {}

impl fmt::Display for NetworkBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkBufferError::BufferEmptyError => write!(f, "Buffer Empty"),
            NetworkBufferError::BufferFullError => write!(f, "Buffer Full"),
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    NoClientAddress,
    MessageTooLarge,
}

impl std::error::Error for ConnectionError {}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionError::NoClientAddress => write!(f, "No client address"),
            ConnectionError::MessageTooLarge => write!(f, "Message too large"),
        }
    }
}
