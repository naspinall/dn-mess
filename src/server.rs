use std::net::SocketAddr;

use crate::{connection::Connection, packets::Frame};

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Server {
    
}

impl Server {
    pub async fn listen(port: &str) -> ServerResult<()> {

        let mut listener = Connection::listen(port).await?;

        loop {
   
            let request_frame = listener.read_frame().await?;
    
            if request_frame.header.recursion_desired {
                let recurse_response = Server::recurse_query(&request_frame).await?;
    
                listener.write_frame(&recurse_response, None).await?;
            }
        }
    }

    pub async fn recurse_query(request: &Frame) -> ServerResult<Frame> {
        // Building query frame to upstream
        let mut recurse_frame = request.build_query();

        let mut recurse_connection =
            Connection::connect(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        for question in request.questions.iter() {
            recurse_frame.add_question(&question)
        }

        // Make request to google
        recurse_connection.write_frame(&recurse_frame, None).await?;

        // Read response
        let mut response_frame = recurse_connection.read_frame().await?;

        response_frame.header.id = request.header.id;

        Ok(response_frame)
    }
}