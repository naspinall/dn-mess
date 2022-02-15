use db::a_records::ARecord;
use log::info;
use packets::Frame;
use server::Server;

mod client;
mod coding;
mod connection;
mod db;
mod errors;
mod network_buffer;
mod packets;
mod server;

fn log_frame(frame: &Frame) {
    info!("{}", frame);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the logger
    env_logger::init();

    let connection = sqlite::open("./db.sqlite")?;

    // Run all the migrations
    db::run_migrations(&connection)?;

    let mut server = Server::new();

    server.add_pre_request_hook(log_frame);
    server.add_post_request_hook(log_frame);

    server.listen(8080).await?;
    Ok(())
}
