use std::{net::SocketAddr};

use tokio::net::UdpSocket;

use crate::{
    coding::FrameCoder, network_buffer::NetworkBuffer, packets::Frame,
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Connection {
    sock: UdpSocket,
    buf: NetworkBuffer,
}

impl Connection {

    pub async fn listen(port: &str) -> ConnectionResult<Connection> {
        // Initializing buffers
        let buf = NetworkBuffer::new();

        // Bind to socket to listen for responses
        let sock = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;

        Ok(Connection {
            sock,
            buf,
        })
    }

    pub async fn write_frame(
        &mut self,
        frame: &Frame,
        addr: &SocketAddr,
    ) -> ConnectionResult<()> {
        
        FrameCoder::new().encode_frame(frame, &mut self.buf)?;

        let buffer_length = self.buf.len();

        self.sock
            .send_to(&self.buf.buf[..buffer_length], addr)
            .await?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self) -> ConnectionResult<(SocketAddr,Frame)> {
        let (_len, addr) = self.sock.recv_from(&mut self.buf.buf).await?;

        let frame = FrameCoder::new().decode_frame(&mut self.buf)?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok((addr,frame))
    }
}
