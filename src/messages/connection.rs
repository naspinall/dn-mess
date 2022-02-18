use std::net::SocketAddr;

use tokio::net::UdpSocket;

use super::{coding::MessageCoder, network_buffer::NetworkBuffer, packets::Message};

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

    pub async fn write_message(
        &mut self,
        sock: &UdpSocket,
        message: &Message,
        to_addr: &SocketAddr,
    ) -> ConnectionResult<usize> {
        // Encode the message, MessageCoder instances should be ephemeral
        MessageCoder::new().encode_message(message, &mut self.buf)?;

        // Only write the length of the buffer
        let buffer_length = self.buf.write_count();

        let write_count = sock
            .send_to(&self.buf.buf[..buffer_length], to_addr)
            .await?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok(write_count)
    }

    pub async fn read_message(
        &mut self,
        sock: &UdpSocket,
    ) -> ConnectionResult<(SocketAddr, Message)> {
        // Read datagram from socket
        let (_len, addr) = sock.recv_from(&mut self.buf.buf).await?;

        // Decode message
        let message = MessageCoder::new().decode_message(&mut self.buf)?;

        // Reset buffer for reuse
        self.buf.reset();

        // Return the remote address and message
        Ok((addr, message))
    }
}
