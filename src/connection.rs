use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    coding::{
        frame_decoder::{decode_answer, decode_header, decode_question},
        frame_encoder::{encode_answer, encode_header, encode_question},
    },
    errors::ConnectionError,
    network_buffer::NetworkBuffer,
    packets::{AnswerPacket, HeaderPacket, QuestionPacket},
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

struct Frame {
    header: HeaderPacket,
    questions: Vec<QuestionPacket>,
    answers: Vec<AnswerPacket>,
}

struct Connection {
    sock: tokio::net::UdpSocket,
    addr: Option<SocketAddr>,
    buf: NetworkBuffer,
}

impl Connection {
    pub fn new(sock: UdpSocket) -> Connection {
        // Initializing buffers
        let buf = NetworkBuffer::new();

        return Connection {
            sock,
            addr: None,
            buf,
        };
    }

    pub async fn write_frame(&mut self, frame: Frame) -> ConnectionResult<()> {
        let addr = match self.addr {
            Some(addr) => addr,
            None => return Err(Box::new(ConnectionError::NoClientAddress)),
        };

        encode_header(&frame.header, &mut self.buf)?;

        // Encode question
        for question in frame.questions.iter() {
            encode_question(question, &mut self.buf)?;
        }

        // Encode question
        for question in frame.questions.iter() {
            encode_question(question, &mut self.buf)?;
        }

        // Encode question
        for answer in frame.answers.iter() {
            encode_answer(answer, &mut self.buf)?;
        }

        self.sock.send_to(&mut self.buf.buf, addr).await?;

        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(sock: UdpSocket) -> ConnectionResult<Frame> {
        let mut network_buffer = NetworkBuffer::new();

        let (len, addr) = sock.recv_from(&mut network_buffer.buf).await?;

        let header = decode_header(&mut network_buffer)?;

        let mut frame = Frame {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
        };

        // Decode question
        for n in 0..=frame.header.question_count {
            let question = decode_question(&mut network_buffer)?;
            frame.questions.push(question);
        }

        // Decode the answer
        for n in 0..=frame.header.answer_count {
            let answer = decode_answer(&mut network_buffer)?;
            frame.answers.push(answer);
        }

        network_buffer.reset();

        return Ok(frame);
    }
}
