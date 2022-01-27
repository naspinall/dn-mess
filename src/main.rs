use connection::Connection;
use tokio::net::UdpSocket;

use crate::packets::{QuestionClass, QuestionPacket, ResourceRecordPacket};

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

        let answer = ResourceRecordPacket {
            domain: client_question.domain.clone(),
            record_type: packets::ResourceRecordType::ARecord,
            class: packets::ResourceRecordClass::InternetAddress,
            time_to_live: 300,
            record_data: packets::ResourceRecordData::ARecord(0x08080808),
        };

        let question = QuestionPacket {
            domain: client_question.domain.clone(),
            class: QuestionClass::InternetAddress,
            question_type: packets::QuestionType::ARecord,
        };

        frame.header.question_count = 1;
        frame.header.answer_count = 1;

        frame.add_question(question);
        frame.add_answer(answer);

        connection.write_frame(frame).await?;
    }
}
