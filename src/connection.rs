use std::net::{SocketAddr};

use tokio::net::UdpSocket;

use crate::{
    coding::FrameCoder,
    errors::ConnectionError,
    network_buffer::NetworkBuffer,
    packets::{HeaderPacket, QuestionPacket, ResourceRecordPacket},
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Frame {
    pub header: HeaderPacket,
    pub questions: Vec<QuestionPacket>,
    pub answers: Vec<ResourceRecordPacket>,
}

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
            addr : Some(addr),
            buf,
            encoder,
        })
    }

    pub async fn listen(port: &str) -> ConnectionResult<Connection> {
        
        // Initializing buffers
        let buf = NetworkBuffer::new();
        let encoder = FrameCoder::new();
        
        // Bind to socket to listen for responses
        let sock = UdpSocket::bind(format!("0.0.0.0.{}",port)).await?;

        Ok(Connection {
            sock,
            addr : None,
            buf,
            encoder,
        })


    }

    pub async fn write_frame(
        &mut self,
        frame: Frame,
        addr: Option<SocketAddr>,
    ) -> ConnectionResult<()> {
        let write_addr = match self.addr {
            Some(addr) => addr,
            None => match addr {
                Some(addr) => addr,
                None => return Err(Box::new(ConnectionError::NoClientAddress)),
            },
        };

        self.encoder.encode_header(&frame.header, &mut self.buf)?;

        // Encode question
        for question in frame.questions.iter() {
            self.encoder.encode_question(question, &mut self.buf)?;
        }

        // Encode question
        for answer in frame.answers.iter() {
            self.encoder.encode_answer(answer, &mut self.buf)?;
        }

        let buffer_length = self.buf.len();

        self.sock
            .send_to(&self.buf.buf[..buffer_length], write_addr)
            .await?;

        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self) -> ConnectionResult<Frame> {
        let (_len, addr) = self.sock.recv_from(&mut self.buf.buf).await?;

        let header = self.encoder.decode_header(&mut self.buf)?;

        let mut frame = Frame {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
        };

        // Decode question
        for _ in 0..frame.header.question_count {
            let question = self.encoder.decode_question(&mut self.buf)?;
            frame.questions.push(question);
        }

        // Decode the answer
        for _ in 0..frame.header.answer_count {
            let answer = self.encoder.decode_resource_record(&mut self.buf)?;
            frame.answers.push(answer);
        }

        self.addr = Some(addr);

        Ok(frame)
    }
}
