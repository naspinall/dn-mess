use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    coding::FrameCoder, errors::ConnectionError, network_buffer::NetworkBuffer, packets::Frame,
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Connection {
    sock: UdpSocket,
    addr: Option<SocketAddr>,
    buf: NetworkBuffer,
    encoder: FrameCoder,
}

impl Connection {
    pub async fn connect(addr: SocketAddr) -> ConnectionResult<Connection> {
        // Initializing buffers
        let buf = NetworkBuffer::new();
        let encoder = FrameCoder::new();

        // Bind to socket to listen for responses
        let sock = UdpSocket::bind("0.0.0.0:0").await?;

        Ok(Connection {
            sock,
            addr: Some(addr),
            buf,
            encoder,
        })
    }

    pub async fn listen(port: &str) -> ConnectionResult<Connection> {
        // Initializing buffers
        let buf = NetworkBuffer::new();
        let encoder = FrameCoder::new();

        // Bind to socket to listen for responses
        let sock = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;

        Ok(Connection {
            sock,
            addr: None,
            buf,
            encoder,
        })
    }

    pub async fn write_frame(
        &mut self,
        frame: &Frame,
        addr: Option<SocketAddr>,
    ) -> ConnectionResult<()> {
        let write_addr = match self.addr {
            Some(addr) => addr,
            None => match addr {
                Some(addr) => addr,
                None => return Err(Box::new(ConnectionError::NoClientAddress)),
            },
        };

        self.encoder.encode_frame(frame, &mut self.buf)?;

        let buffer_length = self.buf.len();

        self.sock
            .send_to(&self.buf.buf[..buffer_length], write_addr)
            .await?;

        // Reset buffer for reuse
        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self) -> ConnectionResult<Frame> {
        let (_len, addr) = self.sock.recv_from(&mut self.buf.buf).await?;

        let frame = self.encoder.decode_frame(&mut self.buf)?;

        self.addr = Some(addr);

        // Reset buffer for reuse
        self.buf.reset();

        Ok(frame)
    }

    pub async fn recurse_query(&self, request: &Frame) -> ConnectionResult<Frame> {
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
