use std::{net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::Mutex};

mod cache;

use crate::{
    client::Client,
    connection::Connection,
    packets::{Frame, ResourceRecord},
};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;
type Cache = Arc<HashCache>;
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

        // Setup access to hash map
        let cache: Cache = Arc::new(HashCache::new());

        loop {
            // Get a reference counted copy of the sockets
            let socket = socket.clone();

            // Wait for an incoming frame
            let (addr, request) = Connection::new().read_frame(&socket).await?;

            // Get a reference counted version of the cache
            let cache = cache.clone();

            // Spawn a new task and move all scoped variables into the task
            tokio::spawn(async move {
                // Handle the request, log any errors
                let response = match Server::handle(&request, &cache).await {
                    Err(error) => {
                        panic!("{}", error)
                    }
                    Ok(response) => response,
                };

                // Write response to socket
                Connection::new()
                    .write_frame(&socket, &response, &addr)
                    .await;
            });
        }
    }

    pub async fn handle(request: &Frame, cache: &Cache) -> ServerResult<Frame> {
        // Create response
        let mut response = request.build_response();

        let mut recurse_request = request.build_query(request.id);

        let (cache_answers, remaining_questions) = cache.get_intersection(&request.questions).await;

        for question in remaining_questions.iter() {
            recurse_request.add_question(question);
        }

        if request.recursion_desired && remaining_questions.len() > 0 {
            // Recurse to get answers
            let upstream_answers = Server::recurse_query(&recurse_request).await?;

            for answer in upstream_answers.iter() {
                response.add_answer(answer);
            }

            // Add upstream answers to the cache
            cache.put_resource_records(&upstream_answers).await;
        }

        for question in request.questions.iter() {
            response.add_question(question);
        }

        for answer in cache_answers.iter() {
            response.add_answer(answer);
        }

        Ok(response)
    }

    pub async fn recurse_query(request: &Frame) -> ServerResult<Vec<ResourceRecord>> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        Ok(response.answers)
    }
}
