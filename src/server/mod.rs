use log::{error, info};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{join, net::UdpSocket};

pub mod cache;
pub mod errors;

use crate::messages::{
    client::Client,
    connection::Connection,
    packets::{Message, ResourceRecordData, ResourceRecordType, ResponseCode},
    Request, Response,
};

use self::{cache::HashCache, errors::RecurseError};

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;
type Cache = Arc<HashCache>;

pub struct Server {
    base_handler: BaseHandler,
}

#[derive(Debug, Clone)]
pub struct BaseHandler {
    cache: Cache,
}

impl BaseHandler {
    fn new() -> BaseHandler {
        BaseHandler {
            cache: Arc::new(HashCache::new()),
        }
    }

    fn cache_records(&self, message: Message) {
        // Get reference counted cache
        let write_cache = self.cache.clone();

        // Put all message resource records at once
        tokio::spawn(async move {
            join!(
                write_cache.put_resource_records(&message.answers),
                write_cache.put_resource_records(&message.authorities),
                write_cache.put_resource_records(&message.additional_records),
            )
        });
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
                // Get the first record, if none break here and continue
                let ns_record = match ns_records.first() {
                    Some(record) => record,
                    None => break,
                };

                // TODO put this as a response method
                let name_server_domain = match &ns_record.data {
                    ResourceRecordData::NS(domain) => domain,
                    _ => break,
                };

                if let Some(a_records) = self
                    .cache
                    .get(ResourceRecordType::ARecord, &name_server_domain)
                    .await
                {
                    // Get the first record, if none break here and continue
                    let a_record = match a_records.first() {
                        Some(record) => record,
                        None => break,
                    };

                    match a_record.data {
                        // Set the name server address to the new address
                        crate::messages::packets::ResourceRecordData::A(value) => {
                            name_server_address.set_ip(IpAddr::V4(Ipv4Addr::from(value)))
                        }
                        _ => break,
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
            let a_record =
                match response.get_record(&ResourceRecordType::ARecord, &name_server_domain) {
                    // If an A record is provided in the response, then use that
                    Some(record) => record.clone(),
                    // Perform another query if not
                    None => {
                        let response = client
                            .query(&name_server_domain, ResourceRecordType::ARecord)
                            .await?;

                        let message = response
                            .get_record(&ResourceRecordType::ARecord, &name_server_domain)
                            .ok_or_else(|| RecurseError::NoARecordError)?
                            .clone();

                        self.cache_records(response);

                        message
                    }
                };

            // Get IP address from A record
            match a_record.data {
                // Set the name server address to the new address
                crate::messages::packets::ResourceRecordData::A(value) => {
                    name_server_address.set_ip(IpAddr::V4(Ipv4Addr::from(value)))
                }
                _ => return Err(Box::new(RecurseError::NoARecordError)),
            };

            // Cache all values
            self.cache_records(response);
        }

        // Finally get the A record
        let client = Client::dial(name_server_address).await?;

        client
            .query(&search_domain, ResourceRecordType::ARecord)
            .await
    }

    async fn handle(&self, request: &Request, mut response: Response) -> ServerResult<Response> {
        let question = match request.questions().first() {
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

                    // Set answers
                    response.set_answers(recurse_response.answers.clone());

                    // Cache response
                    self.cache_records(recurse_response);

                    return Ok(response);
                }

                return Ok(response);
            }
        }
    }
}

impl Server {
    pub async fn new() -> Server {
        Server {
            base_handler: BaseHandler::new(),
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

            let base_handler = self.base_handler.clone();

            // Spawn a new task and move all scoped variables into the task
            tokio::spawn(async move {
                let request = Request::new(message);

                Server::log_message(request.message());

                let mut response = request.response();

                response = match base_handler.handle(&request, response).await {
                    Ok(response) => response,
                    Err(err) => {
                        error!("Handler error {:?}", err);

                        let mut response = request.response();
                        response.set_code(ResponseCode::ServerError);
                        response
                    }
                };

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
