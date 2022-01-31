use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    coding::FrameCoder, errors::ConnectionError, network_buffer::NetworkBuffer, packets::Frame,
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Connection {
    sock: UdpSocket,
    addr: Option<SocketAddr>,
    buf: NetworkBuffer,
}

impl Connection {
    pub async fn connect(addr: SocketAddr) -> ConnectionResult<Connection> {
        // Initializing buffers
        let buf = NetworkBuffer::new();

        // Bind to socket to listen for responses
        let sock = UdpSocket::bind("0.0.0.0:0").await?;

        Ok(Connection {
            sock,
            addr: Some(addr),
            buf,
        })
    }

    pub async fn listen(port: &str) -> ConnectionResult<Connection> {
        // Initializing buffers
        let buf = NetworkBuffer::new();

        // Bind to socket to listen for responses
        let sock = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;

        Ok(Connection {
            sock,
            addr: None,
            buf,
        })
    }

    pub async fn write_frame(
        &mut self,
        frame: &Frame,
        addr: Option<SocketAddr>,
    ) -> ConnectionResult<()> {
        let write_addr = match self.addr {
            Some(addr) => addr,
            None => match addr {
                Some(addr) => addr,
                None => return Err(Box::new(ConnectionError::NoClientAddress)),
            },
        };

        FrameCoder::new().encode_frame(frame, &mut self.buf)?;

        let buffer_length = self.buf.len();

        self.sock
            .send_to(&self.buf.buf[..buffer_length], write_addr)
            .await?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self) -> ConnectionResult<Frame> {
        let (_len, addr) = self.sock.recv_from(&mut self.buf.buf).await?;

        let frame = FrameCoder::new().decode_frame(&mut self.buf)?;

        self.addr = Some(addr);

        // Reset buffer for reuse
        self.buf.reset();

        Ok(frame)
    }
}
