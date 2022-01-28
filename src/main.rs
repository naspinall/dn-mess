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
    let sock = UdpSocket::bind("127.0.0.1:8080").await?;

    let google_dns_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);

    loop {
        // Socket for incoming connections
        let mut connection = Connection::new(&sock);

        let frame = connection.read_frame().await?;

        println!("{:?}", frame);

        // Forward to google
        let outgoing_sock = UdpSocket::bind("0.0.0.0:0").await?;

        let mut outgoing_connection = Connection::new(&outgoing_sock);

        outgoing_connection
            .write_frame(frame, Some(google_dns_address.clone()))
            .await?;

        let frame = outgoing_connection.read_frame().await?;

        println!("{:?}", frame);

        connection.write_frame(frame, None).await?;
    }
}
