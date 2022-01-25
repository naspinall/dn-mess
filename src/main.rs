use connection::Connection;
use tokio::net::UdpSocket;

use crate::packets::{AnswerPacket, QuestionClass, QuestionPacket};

mod coding;
mod connection;
mod errors;
mod network_buffer;
mod packets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sock = UdpSocket::bind("127.0.0.1:8080").await?;

    loop {
        let mut connection = Connection::new(&sock);

        let mut frame = connection.read_frame().await?;

        let client_question = match frame.questions.pop() {
            Some(question) => question,
            None => continue,
        };

        let answer = AnswerPacket {
            domain: client_question.domain.clone(),
            answer_type: client_question.question_type,
            class: client_question.class,
            time_to_live: 300,
            answer_data: packets::AnswerData::ARecord(0x08080808),
        };

        let question = QuestionPacket {
            domain: client_question.domain.clone(),
            class: QuestionClass::InternetAddress,
            question_type: packets::QuestionType::ARecord,
        };

        frame.header.answer_count = 1;

        frame.add_question(question);
        frame.add_answer(answer);

        connection.write_frame(frame).await?;
    }
}
