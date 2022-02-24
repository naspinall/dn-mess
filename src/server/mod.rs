use async_trait::async_trait;
use log::{debug, error, info};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;

pub mod cache;
pub mod errors;

use crate::messages::{
    client::Client,
    connection::Connection,
    packets::{Message, ResourceRecordData, ResourceRecordType},
    Request, Response,
};

use self::{cache::HashCache, errors::RecurseError};

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

    async fn recurse_request(&self, name: &str) -> ServerResult<Message> {
        // Address for the root server
        let mut name_server_address = SocketAddr::from(([198, 41, 0, 4], 53));

        // Split the labels, reverse as we recurse from the base
        let labels = name.split('.').rev();

        let mut search_domain = String::from("");

        for label in labels {
            // Ignore if empty
            if label.is_empty() {
                continue;
            }

            // Append label to the search domain
            search_domain = label.to_owned() + "." + &search_domain;

            // Check if we have a cached NS record
            if let Some(ns_records) = self
                .cache
                .get(ResourceRecordType::NSRecord, &search_domain)
                .await
            {
                // Get the first record, if none just continue
                // TODO bad unwrap
                let ns_record = ns_records.first().unwrap();

                let name_server_domain = match &ns_record.data {
                    ResourceRecordData::NS(domain) => domain,
                    _ => return Err(Box::new(RecurseError::NoNameServerError)),
                };

                if let Some(a_records) = self
                    .cache
                    .get(ResourceRecordType::ARecord, &name_server_domain)
                    .await
                {
                    let a_record = a_records.first().unwrap();

                    match a_record.data {
                        // Set the name server address to the new address
                        crate::messages::packets::ResourceRecordData::A(value) => {
                            name_server_address.set_ip(IpAddr::V4(Ipv4Addr::from(value)))
                        }
                        _ => return Err(Box::new(RecurseError::NoARecordError)),
                    };

                    // We have a cached value, continue on
                    continue;
                }
            }

            let client = Client::dial(name_server_address).await?;

            let response = client
                .query(&search_domain, ResourceRecordType::NSRecord)
                .await?;

            // Get NS record for the search domain
            let ns_record = response
                .get_record(&ResourceRecordType::NSRecord, &search_domain)
                .ok_or_else(|| RecurseError::NoNameServerError)?;

            // Get domain for name server
            let name_server_domain = match &ns_record.data {
                ResourceRecordData::NS(domain) => domain,
                _ => return Err(Box::new(RecurseError::NoNameServerError)),
            };

            // Get an A record for the name server if provided
            let a_record = response
                .get_record(&ResourceRecordType::ARecord, &name_server_domain)
                .ok_or_else(|| RecurseError::NoARecordError)?;

            // Get IP address from A record
            match a_record.data {
                // Set the name server address to the new address
                crate::messages::packets::ResourceRecordData::A(value) => {
                    name_server_address.set_ip(IpAddr::V4(Ipv4Addr::from(value)))
                }
                _ => return Err(Box::new(RecurseError::NoARecordError)),
            };

            let write_cache = self.cache.clone();
            let current_name_server_domain = name_server_domain.clone();
            let current_search_domain = search_domain.clone();

            info!("Writing {} NS Records to cache", current_search_domain);
            info!("Writing {} A Records to cache", current_name_server_domain);

            tokio::spawn(async move {
                write_cache.put_resource_records(&response.answers).await;

                write_cache
                    .put_resource_records(&response.authorities)
                    .await;

                write_cache
                    .put_resource_records(&response.additional_records)
                    .await
            });
        }

        // Finally get the A record
        let client = Client::dial(name_server_address).await?;

        client
            .query(&search_domain, ResourceRecordType::ARecord)
            .await
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
                    // Recurse the request
                    let recurse_response = self.recurse_request(&question.domain).await?;

                    let cache_answers = recurse_response.answers.clone();
                    let write_cache = self.cache.clone();

                    // Set answers
                    response.set_answers(recurse_response.answers);

                    tokio::spawn(async move {
                        write_cache.put_resource_records(&cache_answers).await;
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
