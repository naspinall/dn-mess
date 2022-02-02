use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{connection::Connection, packets::Frame};

type ClientResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Client {
    addr: SocketAddr,
    sock: UdpSocket,
    connection: Connection,
}

impl Client {
    pub async fn dial(addr: SocketAddr) -> ClientResult<Client> {
        // Bind our socket
        let sock = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?;

        // Connect socket to address, so we only receive messages from that address
        sock.connect(addr).await?;

        Ok(Client {
            addr,
            sock,
            connection: Connection::new(),
        })
    }

    pub async fn send(&mut self, frame: &Frame) -> ClientResult<Frame> {
        // Write frame to downstream
        self.connection
            .write_frame(&self.sock, frame, &self.addr)
            .await?;

        // Ignore address as we have connected.
        let (_, frame) = self.connection.read_frame(&self.sock).await?;

        Ok(frame)
    }
}
