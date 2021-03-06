use core::fmt;

#[derive(Debug)]
pub enum NetworkBufferError {
    BufferFullError,
    BufferEmptyError,
    InvalidPacket,
    CompressionError,
    InvalidLabelLengthError(String),
    InvalidNameLengthError(String),
}

impl std::error::Error for NetworkBufferError {}

impl fmt::Display for NetworkBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkBufferError::BufferEmptyError => write!(f, "Buffer Empty"),
            NetworkBufferError::BufferFullError => write!(f, "Buffer Full"),
            NetworkBufferError::InvalidPacket => write!(f, "Invalid Packet"),
            NetworkBufferError::CompressionError => write!(f, "Compression Error"),
            NetworkBufferError::InvalidLabelLengthError(value) => {
                write!(f, "Invalid Label Length: {}", value)
            }
            NetworkBufferError::InvalidNameLengthError(value) => {
                write!(f, "Invalid Name Length: {}", value)
            }
        }
    }
}

#[derive(Debug)]
pub enum ClientError {}

impl std::error::Error for ClientError {}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connection error")
    }
}
