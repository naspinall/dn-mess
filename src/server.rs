use std::{collections::HashMap, net::SocketAddr, sync::Arc, sync::Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

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

        // Wrap socket in reference count for use in both async moves
        let socket = Arc::new(UdpSocket::bind(listen_addr).await?);

        // Create channel to send responses
        let (tx, mut rx) = mpsc::channel(32);

        let (read_socket, write_socket) = (socket.clone(), socket.clone());

        // Setup access to hash map
        let cache: Cache = Arc::new(Mutex::new(HashCache::new()));

        // Spawn receiver to send values down UDP socket
        tokio::spawn(async move {
            while let Some((frame, addr)) = rx.recv().await {
                Connection::new()
                    .write_frame(&write_socket, &frame, &addr)
                    .await;
            }
        });

        loop {
            // Wait for an incoming frame
            let (addr, request) = Connection::new().read_frame(&read_socket).await?;

            let cache = cache.clone();

            let response_tx = tx.clone();

            tokio::spawn(async move {
                // Handle the request, log any errors
                let response = match Server::handle(&request, cache).await {
                    Err(error) => {
                        panic!("{}", error)
                    }
                    Ok(response) => response,
                };

                // Send response down response channel
                response_tx.send((response, addr)).await;
            });
        }
    }

    pub async fn handle(request: &Frame, cache: Cache) -> ServerResult<Frame> {
        // Create response
        let mut response = request.build_response();

        let mut recurse_request = request.build_query(request.id);

        for question in request.questions.iter() {
            recurse_request.add_question(question);
        }

        // Recurse to get answers
        let answers = Server::recurse_query(&recurse_request).await?;

        for question in request.questions.iter() {
            response.add_question(question);
        }

        for answer in answers.iter() {
            response.add_answer(answer);
        }

        Ok(response)
    }

    pub async fn recurse_query(request: &Frame) -> ServerResult<Vec<ResourceRecordPacket>> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        println!("{:?}", response);

        Ok(response.answers)
    }
}
