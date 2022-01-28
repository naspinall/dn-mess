use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use connection::Connection;
use tokio::net::UdpSocket;

use crate::packets::{QuestionClass, QuestionPacket, ResourceRecordPacket};

mod coding;
mod connection;
mod errors;
mod network_buffer;
mod packets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
 
    loop {
        let google_dns_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
        
        // Socket for incoming connections
        let mut connection = Connection::listen("8080").await?;

        let frame = connection.read_frame().await?;

        let mut outgoing_connection = Connection::connect(google_dns_address).await?;

        outgoing_connection
            .write_frame(frame, None)
            .await?;

        let frame = outgoing_connection.read_frame().await?;

        connection.write_frame(frame, None).await?;
    }
}
