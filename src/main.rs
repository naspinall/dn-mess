use server::Server;

mod messages;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the logger
    env_logger::init();

    let server = Server::new();

    server.listen(8080).await
}
