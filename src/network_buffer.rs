const MAX_MESSAGE_SIZE: usize = 512;

type BufferResult<T> = Result<T, &'static str>;
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

    pub fn set_write_cursor(&mut self, cursor: usize) {
        self.write_cursor = cursor;
    }

    pub fn put_u8(&mut self, byte: u8) -> BufferResult<()> {
        // Checking bounds
        if self.write_cursor == MAX_MESSAGE_SIZE {
            return Err("BUFFER FULL");
        }

        // Write the byte
        self.buf[self.write_cursor] = byte;

        // Increment index
        self.write_cursor += 1;

        return Ok(());
    }

    pub fn put_u16(&mut self, value: u16) -> BufferResult<()> {
        if self.write_cursor + 2 == MAX_MESSAGE_SIZE {
            return Err("BUFFER FULL");
        }

        self.buf[self.write_cursor] = (value >> 8) as u8;
        self.buf[self.write_cursor + 1] = (value & 0x00FF) as u8;

        self.write_cursor += 2;

        return Ok(());
    }

    pub fn get_u8(&mut self) -> BufferResult<u8> {
        // Checking bounds
        if self.read_cursor + 1 >= MAX_MESSAGE_SIZE {
            return Err("BUFFER EMPTY");
        }

        let byte = self.buf[self.read_cursor];

        self.read_cursor += 1;

        return Ok(byte);
    }

    pub fn get_u16(&mut self) -> BufferResult<u16> {
        // Checking bounds
        if self.read_cursor + 2 >= MAX_MESSAGE_SIZE {
            return Err("BUFFER EMPTY");
        }

        let value =
            (self.buf[self.read_cursor] as u16) << 8 | self.buf[self.read_cursor + 1] as u16;

        self.read_cursor += 2;

        return Ok(value);
    }
}
