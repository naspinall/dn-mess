use log::{error, info};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};

mod cache;

use crate::{
    client::Client,
    connection::Connection,
    packets::{Frame, ResourceRecord},
};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;
type Cache = Arc<HashCache>;
type FrameHook = fn(&Frame);
pub struct Server {
    pre_request_hooks: Vec<FrameHook>,
    post_request_hooks: Vec<FrameHook>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            pre_request_hooks: vec![],
            post_request_hooks: vec![],
        }
    }

    pub fn add_pre_request_hook(&mut self, hook: FrameHook) {
        self.pre_request_hooks.push(hook);
    }

    pub fn add_post_request_hook(&mut self, hook: FrameHook) {
        self.post_request_hooks.push(hook);
    }

    pub async fn listen(self, port: u16) -> ServerResult<()> {
        // Listen on given port
        let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));

        info!("Listening on {}", listen_addr);

        // Wrap socket in reference count for use in both async moves
        let socket = Arc::new(UdpSocket::bind(listen_addr).await?);

        let write_sock = socket.clone();

        let (response_rx, mut response_tx) = mpsc::channel(1024);

        // Setup access to hash map
        let cache: Cache = Arc::new(HashCache::new());

        tokio::spawn(async move {
            while let Some((response, addr)) = response_tx.recv().await {
                self.post_request_hooks
                    .iter()
                    .for_each(|func| func(&response));

                // Write response to socket
                if let Some(err) = Connection::new()
                    .write_frame(&write_sock, &response, &addr)
                    .await
                    .err()
                {
                    error!("Error writing response {}: {}", response.id, err);
                }
            }
        });

        loop {
            // Get a reference counted copy of the sockets
            let socket = socket.clone();

            // Wait for an incoming frame
            let (addr, request) = Connection::new().read_frame(&socket).await?;

            // Get a reference counted version of the cache
            let cache = cache.clone();

            let rx = response_rx.clone();

            self.pre_request_hooks
                .iter()
                .for_each(|func| func(&request));

            // Spawn a new task and move all scoped variables into the task
            tokio::spawn(async move {
                let id = request.id;

                // Handle the request, log any errors
                let response = match Server::handle(request, &cache).await {
                    Err(err) => {
                        error!("Error handling request {}: {}", id, err);
                        return;
                    }
                    Ok(response) => response,
                };

                if let Some(err) = rx.send((response, addr)).await.err() {
                    error!("Error sending {} down response channel: {}", id, err);
                };
            });
        }
    }

    pub async fn handle(request: Frame, cache: &Cache) -> ServerResult<Frame> {
        // Create response
        let mut response = request.build_response();

        let mut recurse_request = request.build_query(request.id);

        let (cache_answers, remaining_questions) = cache.get_intersection(&request.questions).await;

        for question in remaining_questions.iter() {
            recurse_request.add_question(question);
        }

        if request.recursion_desired && !remaining_questions.is_empty() {
            // Recurse to get answers
            let (upstream_answers, upstream_name_servers) =
                Server::recurse_query(&recurse_request).await?;

            for answer in upstream_answers.iter() {
                response.add_answer(answer);
            }

            for name_server in upstream_name_servers.iter() {
                response.add_name_server(name_server);
            }

            // Set for all questions, will need to remove support for multiple questions
            for question in remaining_questions.iter() {
                // Add upstream answers to the cache
                cache
                    .put_resource_records(
                        &question.domain,
                        &question.question_type,
                        &upstream_answers,
                    )
                    .await;

                cache
                    .put_resource_records(
                        &question.domain,
                        &question.question_type,
                        &upstream_name_servers,
                    )
                    .await;
            }
        }

        response.add_questions(request.questions);
        response.add_answers(cache_answers);

        Ok(response)
    }

    pub async fn recurse_query(
        request: &Frame,
    ) -> ServerResult<(Vec<ResourceRecord>, Vec<ResourceRecord>)> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        Ok((response.answers, response.name_servers))
    }
}
