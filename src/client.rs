use std::net::SocketAddr;

use crate::{connection::Connection, packets::Frame};

type ClientResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Client {
    addr: SocketAddr,
    connection: Connection,
}

impl Client {
    pub async fn dial(addr: SocketAddr) -> ClientResult<Client> {
        // Connect to the given address
        let connection = Connection::connect(&addr).await?;
        Ok(Client { addr, connection })
    }

    pub async fn send(&mut self, frame: &Frame) -> ClientResult<Frame> {
        // Write frame to downstream
        self.connection.write_frame(frame, &self.addr).await?;

        let (_, frame) = self.connection.read_frame().await?;

        Ok(frame)
    }
}
