use std::net::SocketAddr;
use rand::{Rng, prelude::ThreadRng};


mod cache;

use crate::{connection::Connection, packets::Frame};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Server {
    cache: HashCache,
    rng : ThreadRng
}

impl Server {

    pub fn new() -> Server {
        Server {
            rng : rand::thread_rng(),
            cache: HashCache::new(),
        }
    }

    pub async fn listen(&mut self, port: &str) -> ServerResult<()> {

        let mut listener = Connection::listen(port).await?;

        loop {
   
            let (addr, request) = listener.read_frame().await?;

            if request.header.recursion_desired {
                
                let recurse_response = self.recurse_query(&request).await?;
    
                listener.write_frame(&recurse_response, &addr).await?;
            }
        }
    }

    pub async fn recurse_query(&mut self, request: &Frame) -> ServerResult<Frame> {
        // Building query frame to upstream
        let mut recurse_frame = request.build_query(self.rng.gen());

        let mut response_connection = Connection::listen("0").await?;

        let addr = SocketAddr::from(([8, 8, 8, 8], 53));

        for question in request.questions.iter() {
            recurse_frame.add_question(question)
        }

        // Make request to google
        response_connection.write_frame(&recurse_frame, &addr).await?;

        // Read response
        let (_, mut response_frame) = response_connection.read_frame().await?;

        response_frame.header.id = request.header.id;

        Ok(response_frame)
    }
}