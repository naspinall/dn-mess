use async_trait::async_trait;
use log::{error, info};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;

pub mod cache;

use crate::messages::{
    client::Client,
    connection::Connection,
    packets::{Message, ResourceRecord},
    Request, Response,
};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;
type Cache = Arc<HashCache>;

pub trait RequestHandler: Handler + Clone + Send + Sync {}

pub struct Server {
    handlers: Vec<BaseHandler>,
}

#[derive(Clone)]
pub struct BaseHandler {
    cache: Cache,
}

impl BaseHandler {
    fn new() -> BaseHandler {
        BaseHandler {
            cache: Arc::new(HashCache::new()),
        }
    }

    pub async fn recurse_query(
        &self,
        request: &Message,
    ) -> ServerResult<(Vec<ResourceRecord>, Vec<ResourceRecord>)> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        Ok((response.answers, response.name_servers))
    }
}

#[async_trait]
impl Handler for BaseHandler {
    async fn handle(&self, request: &Request, mut response: Response) -> ServerResult<Response> {
        let recurse_request = request.clone();

        let (cache_answers, remaining_questions) =
            self.cache.get_intersection(request.questions()).await;

        // Add cached answers to the response
        for answer in cache_answers {
            response.add_answer(answer)
        }

        if request.recursion_desired() && !remaining_questions.is_empty() {
            // Recurse to get answers
            let (upstream_answers, upstream_name_servers) =
                self.recurse_query(recurse_request.message()).await?;

            for answer in upstream_answers.iter() {
                response.add_answer(answer.clone());
            }

            for name_server in upstream_name_servers.iter() {
                response.add_name_server(name_server.clone());
            }

            // Get a new reference to the cache to move into new task
            let safe_cache = self.cache.clone();

            // Spawn a new async task to set the records in the cache
            tokio::spawn(async move {
                // Set for all questions, will need to remove support for multiple questions
                for question in remaining_questions.iter() {
                    // Add upstream answers to the cache
                    safe_cache
                        .put_resource_records(
                            &question.domain,
                            &question.question_type,
                            &upstream_answers,
                        )
                        .await;

                    safe_cache
                        .put_resource_records(
                            &question.domain,
                            &question.question_type,
                            &upstream_name_servers,
                        )
                        .await;
                }
            });
        }

        Ok(response)
    }
}

#[async_trait]
pub trait Handler {
    async fn handle(&self, request: &Request, mut response: Response) -> ServerResult<Response>;
}

impl Server {
    pub fn new() -> Server {
        Server {
            handlers: vec![BaseHandler::new()],
        }
    }

    fn log_frame(message: &Message) {
        info!("{}", message);
    }

    pub async fn listen(self, port: u16) -> ServerResult<()> {
        // Listen on given port
        let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));

        info!("Listening on {}", listen_addr);

        // Wrap socket in reference count for use in both async moves
        let socket = Arc::new(UdpSocket::bind(listen_addr).await?);

        loop {
            // Get a reference counted copy of the sockets
            let socket = socket.clone();

            // Wait for an incoming message
            let (addr, message) = Connection::new().read_frame(&socket).await?;

            let scoped_handlers = self.handlers.clone();

            // Spawn a new task and move all scoped variables into the task
            tokio::spawn(async move {
                let request = Request::new(addr, message);

                Server::log_frame(request.message());

                let mut response = request.response();

                for handler in scoped_handlers.iter() {
                    response = match handler.handle(&request, response).await {
                        Ok(response) => response,
                        Err(err) => {
                            error!("Handler error {:?}", err);
                            return;
                        }
                    }
                }

                Server::log_frame(response.message());

                // Write response to socket
                if let Some(err) = Connection::new()
                    .write_frame(&socket, response.message(), &addr)
                    .await
                    .err()
                {
                    error!("Error writing response {}: {}", request.id(), err);
                }
            });
        }
    }
}
