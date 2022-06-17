use packets::{Message, PacketType, Question, ResourceRecord};

pub mod client;
mod coding;
pub mod connection;
mod errors;
mod network_buffer;
pub mod packets;

#[derive(Clone)]
pub struct Request {
    message: Message,
}

pub struct Response {
    message: Message,
}

impl Response {
    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn set_answers(&mut self, answers: Vec<ResourceRecord>) {
        self.message.answers = answers
    }
}

impl Request {
    pub fn new(message: Message) -> Request {
        Request { message }
    }

    pub fn id(&self) -> u16 {
        self.message.id
    }

    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn questions(&self) -> &[Question] {
        &self.message.questions
    }

    pub fn recursion_desired(&self) -> bool {
        self.message.recursion_desired
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
