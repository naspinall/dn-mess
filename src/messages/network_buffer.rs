use super::errors::NetworkBufferError;

pub const MAX_MESSAGE_SIZE: usize = 512;

type BufferResult<T> = Result<T, NetworkBufferError>;
pub struct NetworkBuffer {
    pub read_cursor: usize,
    pub write_cursor: usize,
    pub buf: [u8; 512],
}

impl NetworkBuffer {
    pub fn new() -> NetworkBuffer {
        NetworkBuffer {
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

        Ok(())
    }

    pub fn put_u16(&mut self, value: u16) -> BufferResult<usize> {
        if self.write_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[self.write_cursor] = (value >> 8) as u8;
        self.buf[self.write_cursor + 1] = (value & 0x00FF) as u8;

        self.write_cursor += 2;

        Ok(2)
    }

    pub fn set_u16(&mut self, index: usize, value: u16) -> BufferResult<()> {
        if index + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[index] = (value >> 8) as u8;
        self.buf[index + 1] = (value & 0x00FF) as u8;

        Ok(())
    }

    pub fn put_u32(&mut self, value: u32) -> BufferResult<usize> {
        if self.write_cursor + 4 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[self.write_cursor] = (value >> 24) as u8;
        self.buf[self.write_cursor + 1] = (value >> 16) as u8;
        self.buf[self.write_cursor + 2] = (value >> 8) as u8;
        self.buf[self.write_cursor + 3] = (value & 0x00FF) as u8;

        self.write_cursor += 4;

        Ok(4)
    }

    pub fn put_u128(&mut self, value: u128) -> BufferResult<()> {
        if self.write_cursor + 16 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferFullError);
        }

        self.buf[self.write_cursor + 1] = (value >> 112) as u8;
        self.buf[self.write_cursor + 2] = (value >> 104) as u8;
        self.buf[self.write_cursor + 3] = (value >> 96) as u8;
        self.buf[self.write_cursor + 4] = (value >> 88) as u8;
        self.buf[self.write_cursor + 5] = (value >> 80) as u8;
        self.buf[self.write_cursor + 6] = (value >> 72) as u8;
        self.buf[self.write_cursor + 7] = (value >> 64) as u8;
        self.buf[self.write_cursor + 8] = (value >> 56) as u8;
        self.buf[self.write_cursor + 9] = (value >> 48) as u8;
        self.buf[self.write_cursor + 10] = (value >> 40) as u8;
        self.buf[self.write_cursor + 11] = (value >> 32) as u8;
        self.buf[self.write_cursor + 12] = (value >> 24) as u8;
        self.buf[self.write_cursor + 13] = (value >> 16) as u8;
        self.buf[self.write_cursor + 14] = (value >> 8) as u8;
        self.buf[self.write_cursor + 15] = (value & 0x00FF) as u8;

        self.write_cursor += 16;

        Ok(())
    }

    pub fn _put_bytes(&mut self, bytes: &[u8]) -> BufferResult<()> {
        for byte in bytes {
            self.put_u8(*byte)?
        }

        Ok(())
    }

    pub fn get_u8(&mut self) -> BufferResult<u8> {
        // Checking bounds
        if self.read_cursor + 1 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let byte = self.buf[self.read_cursor];

        self.read_cursor += 1;

        Ok(byte)
    }

    pub fn get_u16(&mut self) -> BufferResult<u16> {
        // Checking bounds

        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let value =
            (self.buf[self.read_cursor] as u16) << 8 | self.buf[self.read_cursor + 1] as u16;

        self.read_cursor += 2;

        Ok(value)
    }

    pub fn get_u32(&mut self) -> BufferResult<u32> {
        // Checking bounds
        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let value = (self.buf[self.read_cursor] as u32) << 24
            | (self.buf[self.read_cursor + 1] as u32) << 16
            | (self.buf[self.read_cursor + 2] as u32) << 8
            | self.buf[self.read_cursor + 3] as u32;

        self.read_cursor += 4;

        Ok(value)
    }

    pub fn get_u128(&mut self) -> BufferResult<u128> {
        // Checking bounds
        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err(NetworkBufferError::BufferEmptyError);
        }

        let value = (self.buf[self.read_cursor] as u128) << 120
            | (self.buf[self.read_cursor + 1] as u128) << 112
            | (self.buf[self.read_cursor + 2] as u128) << 104
            | (self.buf[self.read_cursor + 3] as u128) << 96
            | (self.buf[self.read_cursor + 4] as u128) << 88
            | (self.buf[self.read_cursor + 5] as u128) << 80
            | (self.buf[self.read_cursor + 6] as u128) << 72
            | (self.buf[self.read_cursor + 7] as u128) << 64
            | (self.buf[self.read_cursor + 8] as u128) << 56
            | (self.buf[self.read_cursor + 9] as u128) << 48
            | (self.buf[self.read_cursor + 10] as u128) << 40
            | (self.buf[self.read_cursor + 11] as u128) << 32
            | (self.buf[self.read_cursor + 12] as u128) << 24
            | (self.buf[self.read_cursor + 13] as u128) << 16
            | (self.buf[self.read_cursor + 14] as u128) << 8
            | (self.buf[self.read_cursor + 15] as u128);

        self.read_cursor += 16;

        Ok(value)
    }

    pub fn reset(&mut self) {
        self.read_cursor = 0;
        self.write_cursor = 0;
    }

    pub fn write_count(&self) -> usize {
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
        assert_eq!(buf.write_count(), 1);
    }

    #[test]
    fn test_put_u16() {
        let mut buf = NetworkBuffer::new();
        buf.put_u16(0xFC3F).unwrap();

        assert_eq!(buf.buf[0], 0xFC);
        assert_eq!(buf.buf[1], 0x3F);
        assert_eq!(buf.write_count(), 2);
    }

    #[test]
    fn test_put_u32() {
        let mut buf = NetworkBuffer::new();
        buf.put_u32(0x12345678).unwrap();

        assert_eq!(buf.buf[0], 0x12);
        assert_eq!(buf.buf[1], 0x34);
        assert_eq!(buf.buf[2], 0x56);
        assert_eq!(buf.buf[3], 0x78);
        assert_eq!(buf.write_count(), 4);
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
