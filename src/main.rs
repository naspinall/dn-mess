use server::Server;

mod client;
mod coding;
mod connection;
mod errors;
mod network_buffer;
mod packets;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new();
    server.listen("8080").await?;
    Ok(())
}
