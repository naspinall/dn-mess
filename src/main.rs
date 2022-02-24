use server::Server;

mod messages;
mod server;
mod structures;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the logger
    env_logger::init();

    let server = Server::new().await;

    server.listen(8080).await
}
