use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{coding::FrameCoder, network_buffer::NetworkBuffer, packets::Frame};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Connection {
    buf: NetworkBuffer,
}

impl Connection {
    pub fn new() -> Connection {
        // Initializing buffers
        let buf = NetworkBuffer::new();

        Connection { buf }
    }

    pub async fn write_frame(
        &mut self,
        sock: &UdpSocket,
        frame: &Frame,
        to_addr: &SocketAddr,
    ) -> ConnectionResult<()> {
        FrameCoder::new().encode_frame(frame, &mut self.buf)?;

        let buffer_length = self.buf.len();

        sock.send_to(&self.buf.buf[..buffer_length], to_addr)
            .await?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self, sock: &UdpSocket) -> ConnectionResult<(SocketAddr, Frame)> {
        let (_len, addr) = sock.recv_from(&mut self.buf.buf).await?;

        let frame = FrameCoder::new().decode_frame(&mut self.buf)?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok((addr, frame))
    }
}
