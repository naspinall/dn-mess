use crate::errors::NetworkBufferError;

pub const MAX_MESSAGE_SIZE: usize = 512;

type BufferResult<T> = Result<T, NetworkBufferError>;
pub struct NetworkBuffer {
    read_cursor: usize,
    write_cursor: usize,
    pub buf: [u8; 512],
}

impl NetworkBuffer {
    pub fn new() -> Self {
        Self {
            read_cursor: 0,
            write_cursor: 0,
            buf: [0; MAX_MESSAGE_SIZE],
        }
    }

    pub fn put_u8(&mut self, byte: u8) -> BufferResult<()> {
        // Checking bounds
        if self.write_cursor + 1 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        // Write the byte
        self.buf[self.write_cursor] = byte;

        // Increment index
        self.write_cursor += 1;

        return Ok(());
    }

    pub fn put_u16(&mut self, value: u16) -> BufferResult<()> {
        if self.write_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[self.write_cursor] = (value >> 8) as u8;
        self.buf[self.write_cursor + 1] = (value & 0x00FF) as u8;

        self.write_cursor += 2;

        return Ok(());
    }

    pub fn put_u32(&mut self, value: u32) -> BufferResult<()> {
        if self.write_cursor + 4 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[self.write_cursor] = (value >> 24) as u8;
        self.buf[self.write_cursor + 1] = (value >> 16) as u8;
        self.buf[self.write_cursor + 2] = (value >> 8) as u8;
        self.buf[self.write_cursor + 3] = (value & 0x00FF) as u8;

        self.write_cursor += 4;

        return Ok(());
    }

    pub fn get_u8(&mut self) -> BufferResult<u8> {
        // Checking bounds
        if self.read_cursor + 1 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let byte = self.buf[self.read_cursor];

        self.read_cursor += 1;

        return Ok(byte);
    }

    pub fn get_u16(&mut self) -> BufferResult<u16> {
        // Checking bounds
        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let value =
            (self.buf[self.read_cursor] as u16) << 8 | self.buf[self.read_cursor + 1] as u16;

        self.read_cursor += 2;

        return Ok(value);
    }

    pub fn get_u32(&mut self) -> BufferResult<u32> {
        // Checking bounds
        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let value = (self.buf[self.read_cursor] as u32) << 24
            | (self.buf[self.read_cursor + 1] as u32) << 16 as u32
            | (self.buf[self.read_cursor + 2] as u32) << 8
            | self.buf[self.read_cursor + 3] as u32;

        self.read_cursor += 4;

        return Ok(value);
    }

    pub fn reset(&mut self) {
        self.read_cursor = 0;
        self.write_cursor = 0;
    }

    pub fn len(&self) -> usize {
        self.write_cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_u8() {
        let mut buf = NetworkBuffer::new();
        buf.put_u8(0xFF).unwrap();

        assert_eq!(buf.buf[0], 0xFF);
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn test_put_u16() {
        let mut buf = NetworkBuffer::new();
        buf.put_u16(0xFC3F).unwrap();

        assert_eq!(buf.buf[0], 0xFC);
        assert_eq!(buf.buf[1], 0x3F);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_put_u32() {
        let mut buf = NetworkBuffer::new();
        buf.put_u32(0x12345678).unwrap();

        assert_eq!(buf.buf[0], 0x12);
        assert_eq!(buf.buf[1], 0x34);
        assert_eq!(buf.buf[2], 0x56);
        assert_eq!(buf.buf[3], 0x78);
        assert_eq!(buf.len(), 4);
    }

    #[test]
    fn test_put_u8_at_capacity() {
        let mut buf = NetworkBuffer::new();

        buf.write_cursor = MAX_MESSAGE_SIZE;

        assert!(buf.put_u8(0xFF).is_err());

        buf.write_cursor = MAX_MESSAGE_SIZE - 1;

        assert!(buf.put_u8(0xFF).is_err());
    }

    #[test]
    fn test_put_u16_at_capacity() {
        let mut buf = NetworkBuffer::new();

        buf.write_cursor = MAX_MESSAGE_SIZE;
        assert!(buf.put_u16(0xFFFF).is_err());

        buf.write_cursor = MAX_MESSAGE_SIZE - 1;
        assert!(buf.put_u16(0xFFFF).is_err());

        buf.write_cursor = MAX_MESSAGE_SIZE - 2;
        assert!(buf.put_u16(0xFFFF).is_err());
    }

    #[test]
    fn test_put_u32_at_capacity() {
        let mut buf = NetworkBuffer::new();

        buf.write_cursor = MAX_MESSAGE_SIZE;
        assert!(buf.put_u32(0xFFFF).is_err());
        buf.write_cursor = MAX_MESSAGE_SIZE - 1;

        assert!(buf.put_u32(0xFFFF).is_err());
        buf.write_cursor = MAX_MESSAGE_SIZE - 2;

        assert!(buf.put_u32(0xFFFF).is_err());
        buf.write_cursor = MAX_MESSAGE_SIZE - 3;

        assert!(buf.put_u32(0xFFFF).is_err());
        buf.write_cursor = MAX_MESSAGE_SIZE - 4;
        assert!(buf.put_u32(0xFFFF).is_err());
    }

    #[test]
    fn test_get_u8() {
        let mut buf = NetworkBuffer::new();
        buf.buf[0] = 0xFF;

        let value = buf.get_u8().unwrap();

        assert_eq!(value, 0xFF);
        assert_eq!(buf.read_cursor, 1);
    }

    #[test]
    fn test_get_u16() {
        let mut buf = NetworkBuffer::new();
        buf.buf[0] = 0xFF;
        buf.buf[1] = 0x11;

        let value = buf.get_u16().unwrap();

        assert_eq!(value, 0xFF11);
        assert_eq!(buf.read_cursor, 2);
    }

    #[test]
    fn test_get_u32() {
        let mut buf = NetworkBuffer::new();
        buf.buf[0] = 0xFF;
        buf.buf[1] = 0x11;
        buf.buf[2] = 0x22;
        buf.buf[3] = 0x33;

        let value = buf.get_u32().unwrap();

        assert_eq!(value, 0xFF112233);
        assert_eq!(buf.read_cursor, 4);
    }
}
