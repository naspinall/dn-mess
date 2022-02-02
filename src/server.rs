use std::{net::SocketAddr, sync::Arc, sync::Mutex};
use tokio::net::UdpSocket;

mod cache;

use crate::{
    client::Client,
    connection::Connection,
    packets::{Frame, ResourceRecordPacket},
};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;

type Cache = Arc<Mutex<HashCache>>;
pub struct Server {}

impl Server {
    pub fn new() -> Server {
        Server {}
    }

    pub async fn listen(self, port: u16) -> ServerResult<()> {
        // Listen on given port
        let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listen_socket = UdpSocket::bind(listen_addr).await?;

        // Setup access to hash map
        let cache: Cache = Arc::new(Mutex::new(HashCache::new()));

        loop {
            // Wait for an incoming frame
            let (addr, request) = Connection::new().read_frame(&listen_socket).await?;

            let cache = cache.clone();

            tokio::spawn(async move {
                // Perform any required

                // Handle the request, log any errors
                match Server::handle(request, addr, cache).await {
                    Err(error) => {}
                    Ok(request) => {}
                };
            });
        }
    }

    pub async fn handle(
        request: Frame,
        return_addr: SocketAddr,
        cache: Cache,
    ) -> ServerResult<Frame> {
        // Create response
        let mut response = request.build_response();

        // Recurse to get answers
        let answers = Server::recurse_query(&request).await?;

        // Set answers
        response.answers = answers;

        Ok(response)
    }

    pub async fn recurse_query(request: &Frame) -> ServerResult<Vec<ResourceRecordPacket>> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        Ok(response.answers)
    }

    pub fn log_frame(frame: &Frame, addr: &SocketAddr) -> Option<Box<dyn std::error::Error>> {
        let mut log = format!("{:?} {} {}", frame.packet_type, addr, frame.id);

        for question in frame.questions.iter() {
            log.push_str(format!(" {:?} {}", question.question_type, question.domain).as_str());
        }

        for answer in frame.answers.iter() {
            log.push_str(
                format!(
                    " {:?} {} {} {:?}",
                    answer.record_type, answer.domain, answer.time_to_live, answer.record_data
                )
                .as_str(),
            );
        }

        info!("{}", log);

        return None;
    }
}
