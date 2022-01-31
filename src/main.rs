use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use connection::Connection;

mod coding;
mod connection;
mod errors;
mod network_buffer;
mod packets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Socket for incoming connections
        let mut connection = Connection::listen("8080").await?;

        let request_frame = connection.read_frame().await?;

        if request_frame.header.recursion_desired {
            let recurse_response = connection.recurse_query(&request_frame).await?;

            connection.write_frame(&recurse_response, None).await?;
        }
    }
}
