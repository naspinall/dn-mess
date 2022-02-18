use packets::{Message, PacketType, Question, ResourceRecord, ResponseCode};
use std::net::SocketAddr;

pub mod client;
mod coding;
pub mod connection;
mod errors;
mod network_buffer;
pub mod packets;

#[derive(Clone)]
pub struct Request {
    addr: SocketAddr,
    message: Message,
}

pub struct Response {
    message: Message,
}

impl Response {
    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn message_mut(&mut self) -> &mut Message {
        &mut self.message
    }

    pub fn authoritative_answer(&self) -> bool {
        self.message.authoritative_answer
    }
    pub fn truncation(&self) -> bool {
        self.message.truncation
    }
    pub fn recursion_available(&self) -> bool {
        self.message.recursion_available
    }
    pub fn response_code(&self) -> &ResponseCode {
        &self.message.response_code
    }

    pub fn add_answer(&mut self, record: ResourceRecord) {
        self.message.answers.push(record)
    }
    pub fn add_name_server(&mut self, record: ResourceRecord) {
        self.message.authorities.push(record)
    }

    pub fn set_answers(&mut self, answers: Vec<ResourceRecord>) {
        self.message.answers = answers
    }
}

impl Request {
    pub fn new(addr: SocketAddr, message: Message) -> Request {
        Request { addr, message }
    }

    pub fn id(&self) -> u16 {
        self.message.id
    }

    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn questions(&self) -> &[Question] {
        &self.message.questions
    }

    pub fn packet_type(&self) -> &PacketType {
        &self.message.packet_type
    }
    pub fn op_code(&self) -> u8 {
        self.message.op_code
    }
    pub fn recursion_desired(&self) -> bool {
        self.message.recursion_desired
    }

    pub fn add_question(&mut self, question: Question) {
        self.message.questions.push(question)
    }

    pub fn response(&self) -> Response {
        // Clone the current request to preset fields
        let mut message = self.message.clone();

        // Set a response type
        message.packet_type = PacketType::Response;
        message.recursion_available = true;

        Response { message }
    }
}
