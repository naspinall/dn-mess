use std::net::SocketAddr;

use tokio::net::UdpSocket;

use super::{connection::Connection, packets::Message};

type ClientResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Client {
    addr: SocketAddr,
    sock: UdpSocket,
    connection: Connection,
}

impl Client {
    /// Dial and connect to a remote address. The client will only read messages from the given remote address.
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

    pub async fn send(&mut self, message: &Message) -> ClientResult<Message> {
        // Write message to downstream
        self.connection
            .write_message(&self.sock, message, &self.addr)
            .await?;

        // Ignore address as we have connected.
        let (_, message) = self.connection.read_message(&self.sock).await?;

        Ok(message)
    }
}
