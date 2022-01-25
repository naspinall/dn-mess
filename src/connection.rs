use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    coding::{
        frame_decoder::{decode_answer, decode_header, decode_question},
        frame_encoder::{encode_answer, encode_header, encode_question},
    },
    errors::ConnectionError,
    network_buffer::{NetworkBuffer, MAX_MESSAGE_SIZE},
    packets::{AnswerPacket, HeaderPacket, QuestionPacket},
};

type ConnectionResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Frame {
    pub header: HeaderPacket,
    pub questions: Vec<QuestionPacket>,
    pub answers: Vec<AnswerPacket>,
}

impl Frame {
    pub fn add_answer(&mut self, answer: AnswerPacket) {
        self.answers.push(answer);
    }
    pub fn add_question(&mut self, question: QuestionPacket) {
        self.questions.push(question);
    }
}

pub struct Connection<'a> {
    sock: &'a UdpSocket,
    addr: Option<SocketAddr>,
    buf: NetworkBuffer,
}

impl Connection<'_> {
    pub fn new(sock: &UdpSocket) -> Connection {
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
        for answer in frame.answers.iter() {
            encode_answer(answer, &mut self.buf)?;
        }

        self.sock.send_to(&mut self.buf.buf, addr).await?;

        self.buf.reset();

        Ok(())
    }

    pub async fn read_frame(&mut self) -> ConnectionResult<Frame> {
        let (len, addr) = self.sock.recv_from(&mut self.buf.buf).await?;

        let header = decode_header(&mut self.buf)?;

        let mut frame = Frame {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
        };

        // Decode question
        for _ in 0..frame.header.question_count {
            let question = decode_question(&mut self.buf)?;
            frame.questions.push(question);
        }

        // Decode the answer
        for _ in 0..frame.header.answer_count {
            let answer = decode_answer(&mut self.buf)?;
            frame.answers.push(answer);
        }

        self.addr = Some(addr);

        return Ok(frame);
    }
}
