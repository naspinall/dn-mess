use async_trait::async_trait;
use log::{error, info};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::Mutex};

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

pub struct Server {
    handlers: Vec<Arc<BaseHandler>>,
}

pub struct BaseHandler {
    cache: Cache,
    resolver: Client,
}

impl BaseHandler {
    async fn new(resovler_addr: SocketAddr) -> ServerResult<BaseHandler> {
        // Create client for use by handler
        let client = Client::dial(resovler_addr).await?;

        Ok(BaseHandler {
            cache: Arc::new(HashCache::new()),
            resolver: client,
        })
    }
}

#[async_trait]
impl Handler for BaseHandler {
    async fn handle(&self, request: &Request, mut response: Response) -> ServerResult<Response> {
        let question = match request.questions().get(0) {
            // Get first question
            Some(question) => question,

            // If no questions, just return a blank answer
            None => return Ok(response),
        };

        match self
            .cache
            .get(question.question_type.clone(), &question.domain)
            .await
        {
            Some(records) => {
                // Set answers from cache
                response.set_answers(records);

                // Send response
                return Ok(response);
            }
            None => {
                // Check that recursion is required

                if request.recursion_desired() {
                    let recurse_response = self
                        .resolver
                        .query(&question.domain, question.question_type.clone())
                        .await?;

                    let cache_answers = recurse_response.answers.clone();
                    let write_cache = self.cache.clone();
                    let domain = question.domain.clone();

                    // Set answers
                    response.set_answers(recurse_response.answers);

                    tokio::spawn(async move {
                        write_cache
                            .put_resource_records(&domain, &cache_answers)
                            .await;
                    });

                    return Ok(response);
                }

                return Ok(response);
            }
        }
    }
}

#[async_trait]
pub trait Handler {
    async fn handle(&self, request: &Request, mut response: Response) -> ServerResult<Response>;
}

impl Server {
    pub async fn new() -> Server {
        Server {
            handlers: vec![Arc::new(
                BaseHandler::new(SocketAddr::from(([8, 8, 8, 8], 53)))
                    .await
                    .unwrap(),
            )],
        }
    }

    fn log_message(message: &Message) {
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
            let (addr, message) = Connection::new().read_message(&socket).await?;

            let scoped_handlers = self.handlers.clone();

            // Spawn a new task and move all scoped variables into the task
            tokio::spawn(async move {
                let request = Request::new(addr, message);

                Server::log_message(request.message());

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

                Server::log_message(response.message());

                // Write response to socket
                if let Some(err) = Connection::new()
                    .write_message(&socket, response.message(), &addr)
                    .await
                    .err()
                {
                    error!("Error writing response {}: {}", request.id(), err);
                }
            });
        }
    }
}
