use tokio::net::UdpSocket;

use crate::packets::{AnswerPacket, HeaderPacket, PacketType, QuestionClass, QuestionType};

mod coding;
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

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_decode_domain_label() {
        let hello: [u8; 5] = [0x68, 0x65, 0x6c, 0x6c, 0x6f];

        let label = decode_domain_label(5, &hello);

        assert_eq!(label, "hello");
    }

    #[test]
    fn test_decode_single_domain() {
        let hello: [u8; 6] = [0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f];

        let (n, label) = decode_domain(&hello);

        assert_eq!(label, ".hello");
        assert_eq!(n, 6);
    }

    #[test]
    fn test_decode_complicated_domain() {
        let hello: [u8; 13] = [
            0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03, 0x63, 0x6f, 0x6d, 0x02, 0x61, 0x75,
        ];

        let (n, label) = decode_domain(&hello);

        assert_eq!(label, ".hello.com.au");
        assert_eq!(n, 13);
    }
}
