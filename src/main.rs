use tokio::net::UdpSocket;

use crate::packets::{AnswerPacket, HeaderPacket, PacketType, QuestionClass, QuestionType};

mod coding;
mod connection;
mod errors;
mod network_buffer;
mod packets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sock = UdpSocket::bind("127.0.0.1:8080").await?;

    loop {
        let mut buf = network_buffer::NetworkBuffer::new();

        let (len, addr) = sock.recv_from(&mut buf.buf).await?;

        buf.set_write_cursor(len);

        println!("{:?} bytes received from {:?}", len, addr);

        let header = coding::frame_decoder::decode_header(&mut buf)?;
        let question = coding::frame_decoder::decode_question(&mut buf)?;

        let answer_header = HeaderPacket {
            id: header.id,
            packet_type: PacketType::Answer,
            op_code: 0,
            name_server_count: 0,
            question_count: 1,
            answer_count: 1,
            additional_records_count: 0,
        };

        let mut answer_buf = network_buffer::NetworkBuffer::new();

        coding::frame_encoder::encode_header(&answer_header, &mut answer_buf)?;

        println!("{:?}", header);

        println!("{:?}", question);

        coding::frame_encoder::encode_question(&question, &mut answer_buf)?;

        let answer = AnswerPacket {
            domain: question.domain,
            answer_type: QuestionType::ARecord,
            class: QuestionClass::InternetAddress,
            time_to_live: 100,
        };

        println!("{:?}", answer);

        coding::frame_encoder::encode_answer(&answer, &mut answer_buf)?;

        println!("{:?}", buf.buf);
        println!("{:?}", answer_buf.buf);

        let len = sock.send_to(&answer_buf.buf, addr).await?;

        println!("{:?} bytes sent from {:?}", len, addr);
    }
}
