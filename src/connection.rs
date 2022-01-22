struct Frame {}

struct Connection {
    sock: UdpSocket,
    addr: SocketAddr,
    read_buf: NetworkBuffer,
    write_buf: NetworkBuffer,
}

impl Connection {
    pub fn new(sock: UdpSocket, addr: SocketAddr) -> Connection {
        // Initializing buffers
        let read_buf = NetworkBuffer::new();
        let write_buf = NetworkBuffer::new();

        return Connection {
            sock,
            addr,
            read_buf,
            write_buf,
        };
    }

    pub async fn write_frame(&mut self, frame: Frame) {}

    pub async fn read_frame(&mut self, frame: Frame) -> Option<Frame> {
        return Some(Frame {});
    }
}
