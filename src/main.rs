use std::net::SocketAddr;

use tokio::net::UdpSocket;

const HEADER_LENGTH: usize = 12;
const BUFFER_LENGTH: usize = 512;

const MAX_MESSAGE_SIZE: usize = 512;

#[derive(Debug)]
enum PacketType {
    Question,
    Answer,
}

#[derive(Debug)]

enum QuestionType {
    ARecord,
    CNameRecord,
    MXRecord,
    NameServersRecord,
    Unimplemented,
}
#[derive(Debug)]

enum QuestionClass {
    InternetAddress,
    Unimplemented,
}

#[derive(Debug)]
enum AnswerData {
    ARecord(u16),
    CName(String),
}

#[derive(Debug)]
pub struct HeaderPacket {
    id: u16,
    packet_type: PacketType,
    op_code: u8,

    question_count: u16,
    answer_count: u16,
    name_server_count: u16,
    additional_records_count: u16,
}

#[derive(Debug)]
pub struct QuestionPacket {
    domain: String,
    question_type: QuestionType,
    class: QuestionClass,
}

#[derive(Debug)]
pub struct AnswerPacket {
    domain: String,
    answer_type: QuestionType,
    class: QuestionClass,
    time_to_live: u16,
}

type CodingResult<T> = Result<T, &'static str>;

mod frame_encoder {
    use crate::{
        AnswerPacket, CodingResult, HeaderPacket, NetworkBuffer, PacketType, QuestionPacket,
        QuestionType,
    };

    pub fn encode_domain_label(label: &String, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Setting label length
        buf.put_u8(label.len() as u8)?;

        // Add each character
        for character in label.chars() {
            buf.put_u8(character as u8)?;
        }

        Ok(())
    }

    pub fn encode_domain(domain: &String, buf: &mut NetworkBuffer) -> CodingResult<()> {
        let labels = domain.split(".");

        let mut n = 0;

        for label in labels {
            // Skip empty strings
            if label.is_empty() {
                continue;
            }

            encode_domain_label(&label.to_string(), buf)?;
        }

        // Terminating domain name
        buf.put_u8(0x00)
    }

    pub fn encode_answer(answer: &AnswerPacket, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Encode domain name
        let mut n = encode_domain(&answer.domain, buf);

        // Encode type
        let type_bytes: u16 = match answer.answer_type {
            QuestionType::ARecord => 0x0001,
            QuestionType::NameServersRecord => 0x0002,
            QuestionType::CNameRecord => 0x0005,
            QuestionType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        buf.put_u16(type_bytes)?;

        // Encode class
        buf.put_u16(1)?;

        // Encode time to live
        buf.put_u16(0x00)?;
        buf.put_u16(answer.time_to_live)?;

        // Encoding RData length field
        buf.put_u16(0x04)?;

        // Encode RDdata field
        buf.put_u16(0x0808)?;
        buf.put_u16(0x0808)
    }

    pub fn encode_header(header: &HeaderPacket, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Encode the packet ID
        buf.put_u16(header.id)?;

        let mut options: u16 = 0x00;

        options = options
            | match header.packet_type {
                PacketType::Question => 0x00,
                PacketType::Answer => 0x80,
            };

        buf.put_u16(options)?;

        // Ignore other fields for now

        // Encode Question Count
        buf.put_u16(header.question_count)?;

        // Encode Answer Count
        buf.put_u16(header.answer_count)?;

        // Encode Name Server Count
        buf.put_u16(header.name_server_count)?;
        // Encode Additional Records Count

        buf.put_u16(header.additional_records_count)
    }

    pub fn encode_question(question: &QuestionPacket, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Encode domain name
        encode_domain(&question.domain, buf)?;

        // Encode type
        let type_bytes: u16 = match question.question_type {
            QuestionType::ARecord => 0x0001,
            QuestionType::NameServersRecord => 0x0002,
            QuestionType::CNameRecord => 0x0005,
            QuestionType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        buf.put_u16(type_bytes)?;

        // Encode class
        buf.put_u16(1)
    }
}

mod frame_decoder {

    use super::*;

    pub fn decode_header(buf: &mut NetworkBuffer) -> CodingResult<HeaderPacket> {
        // decode ID field
        let id = buf.get_u16()?;

        // decode query response bit
        let packet_type = match 0x1 & buf.get_u8()? == 1 {
            true => PacketType::Question,
            false => PacketType::Answer,
        };

        let op_code = buf.get_u8()? & 0xE << 1;

        let question_count = buf.get_u16()?;
        let answer_count = buf.get_u16()?;
        let name_server_count = buf.get_u16()?;
        let additional_records_count = buf.get_u16()?;

        return Ok(HeaderPacket {
            id,
            packet_type,
            op_code,
            question_count,
            answer_count,
            name_server_count,
            additional_records_count,
        });
    }

    pub fn decode_question(buf: &mut NetworkBuffer) -> CodingResult<QuestionPacket> {
        // Decode the domain
        let domain = decode_domain(buf)?;

        // Decode the type
        let question_type = match buf.get_u16()? {
            0x0001 => QuestionType::ARecord,
            0x0002 => QuestionType::NameServersRecord,
            0x0005 => QuestionType::CNameRecord,
            0x000f => QuestionType::MXRecord,
            _ => QuestionType::Unimplemented,
        };

        // Decode the class
        let class = match buf.get_u16()? {
            0x001 => QuestionClass::InternetAddress,
            _ => QuestionClass::Unimplemented,
        };

        return Ok(QuestionPacket {
            domain,
            question_type,
            class,
        });
    }

    pub fn decode_domain_label(length: usize, buf: &mut NetworkBuffer) -> CodingResult<String> {
        let mut label = String::new();

        let mut n = 0;

        while n < length {
            label.push(buf.get_u8()? as char);
            n = n + 1;
        }

        return Ok(label);
    }

    pub fn decode_domain(buf: &mut NetworkBuffer) -> CodingResult<String> {
        let mut domain = String::new();

        let mut label_length = buf.get_u8()? as usize;

        while label_length != 0x00 {
            // Decode current label
            let label = decode_domain_label(label_length, buf)?;

            // Add separator
            domain.push('.');

            // Add the label to the total domain
            domain.push_str(&label);

            label_length = buf.get_u8()? as usize;
        }

        return Ok(domain);
    }
}

pub struct NetworkBuffer {
    read_cursor: usize,
    write_cursor: usize,
    buf: [u8; 512],
}

impl NetworkBuffer {
    pub fn new() -> Self {
        Self {
            read_cursor: 0,
            write_cursor: 0,
            buf: [0; MAX_MESSAGE_SIZE],
        }
    }

    fn put_u8(&mut self, byte: u8) -> CodingResult<()> {
        // Checking bounds
        if self.write_cursor == BUFFER_LENGTH {
            return Err("BUFFER FULL");
        }

        // Write the byte
        self.buf[self.write_cursor] = byte;

        // Increment index
        self.write_cursor += 1;

        return Ok(());
    }

    fn put_u16(&mut self, value: u16) -> CodingResult<()> {
        if self.write_cursor + 2 == BUFFER_LENGTH {
            return Err("BUFFER FULL");
        }

        self.buf[self.write_cursor] = (value >> 8) as u8;
        self.buf[self.write_cursor + 1] = (value & 0x00FF) as u8;

        self.write_cursor += 2;

        return Ok(());
    }

    fn get_u8(&mut self) -> CodingResult<u8> {
        // Checking bounds
        if self.read_cursor + 1 >= BUFFER_LENGTH {
            return Err("BUFFER EMPTY");
        }

        let byte = self.buf[self.read_cursor];

        self.read_cursor += 1;

        return Ok(byte);
    }

    fn get_u16(&mut self) -> CodingResult<u16> {
        // Checking bounds
        if self.read_cursor + 2 >= BUFFER_LENGTH {
            return Err("BUFFER EMPTY");
        }

        let value =
            (self.buf[self.read_cursor] as u16) << 8 | self.buf[self.read_cursor + 1] as u16;

        self.read_cursor += 2;

        return Ok(value);
    }
}

fn decode_u16(buf: &[u8]) -> u16 {
    (buf[0] as u16) << 8 | buf[1] as u16
}

fn encode_u16(buf: &mut [u8], value: u16) {
    buf[0] = (value >> 8) as u8;
    buf[1] = (value & 0x00FF) as u8;
}

fn decode_header(&buf: &[u8; 1024]) -> HeaderPacket {
    // decode ID field
    let id = decode_u16(&buf);

    // decode query response bit
    let packet_type = match 0x1 & buf[2] == 1 {
        true => PacketType::Question,
        false => PacketType::Answer,
    };

    let op_code = buf[2] & 0xE << 1;

    let question_count = (buf[4] as u16) << 8 | buf[5] as u16;
    let answer_count = (buf[6] as u16) << 8 | buf[7] as u16;
    let name_server_count = (buf[8] as u16) << 8 | buf[9] as u16;
    let additional_records_count = (buf[10] as u16) << 8 | buf[11] as u16;

    return HeaderPacket {
        id,
        packet_type,
        op_code,
        question_count,
        answer_count,
        name_server_count,
        additional_records_count,
    };
}

fn decode_question(buf: &[u8]) -> QuestionPacket {
    // decode the domain

    let (n, domain) = decode_domain(buf);

    // decode the type
    let question_type = match decode_u16(&buf[n + 1..]) {
        0x0001 => QuestionType::ARecord,
        0x0002 => QuestionType::NameServersRecord,
        0x0005 => QuestionType::CNameRecord,
        0x000f => QuestionType::MXRecord,
        _ => QuestionType::Unimplemented,
    };

    // Prase the class
    let class = match decode_u16(&buf[n + 3..]) {
        0x001 => QuestionClass::InternetAddress,
        _ => QuestionClass::Unimplemented,
    };

    return QuestionPacket {
        domain,
        question_type,
        class,
    };
}

fn decode_domain_label(length: usize, buf: &[u8]) -> String {
    let mut label = String::new();

    let mut n = 0;

    while n < length {
        label.push(buf[n] as char);
        n = n + 1;
    }

    return label;
}

fn encode_domain_label(label: &String, buf: &mut [u8]) -> usize {
    // This will overflow, address later
    let length = label.len() as u8;

    // Set the length
    buf[0] = length;

    let mut n = 1;

    // Add each character
    for character in label.chars() {
        buf[n] = character as u8;
        n += 1;
    }

    return n;
}

fn encode_domain(domain: &String, buf: &mut [u8]) -> usize {
    let labels = domain.split(".");

    let mut n = 0;

    for label in labels {
        // Skip empty strings
        if label.is_empty() {
            continue;
        }

        n += encode_domain_label(&label.to_string(), &mut buf[n..]);
    }

    // Terminating domain name
    buf[n + 1] = 0x00;

    return n + 1;
}

fn decode_domain(buf: &[u8]) -> (usize, String) {
    let mut domain = String::new();

    let mut i = 0;

    while i < buf.len() {
        // Length of the current label to be read
        let label_length = buf[i] as usize;

        // Check if we are at the null byte and terminate
        if label_length == 0x00 {
            break;
        }

        let label = decode_domain_label(label_length, &buf[i + 1..]);

        // Add separator
        domain.push('.');

        // Add the label to the total domain
        domain.push_str(&label);

        // Add to the next label in domain
        i += label_length + 1
    }

    return (i, domain);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sock = UdpSocket::bind("127.0.0.1:8080").await?;

    loop {
        let mut buf = NetworkBuffer::new();

        let (len, addr) = sock.recv_from(&mut buf.buf).await?;

        buf.write_cursor = len;

        println!("{:?} bytes received from {:?}", len, addr);

        let header = frame_decoder::decode_header(&mut buf)?;
        let question = frame_decoder::decode_question(&mut buf)?;

        let answer_header = HeaderPacket {
            id: header.id,
            packet_type: PacketType::Answer,
            op_code: 0,
            name_server_count: 0,
            question_count: 1,
            answer_count: 1,
            additional_records_count: 0,
        };

        let mut answer_buf = NetworkBuffer::new();

        frame_encoder::encode_header(&answer_header, &mut answer_buf)?;

        println!("{:?}", header);

        println!("{:?}", question);

        frame_encoder::encode_question(&question, &mut answer_buf)?;

        let answer = AnswerPacket {
            domain: question.domain,
            answer_type: QuestionType::ARecord,
            class: QuestionClass::InternetAddress,
            time_to_live: 100,
        };

        println!("{:?}", answer);

        frame_encoder::encode_answer(&answer, &mut answer_buf)?;

        println!("{:?}", buf.buf);
        println!("{:?}", answer_buf.buf);

        let len = sock.send_to(&answer_buf.buf, addr).await?;

        println!("{:?} bytes sent from {:?}", len, addr);
    }
}

impl HeaderPacket {
    fn encode(&self, buf: &mut [u8]) -> usize {
        // Encode the packet ID
        encode_u16(buf, self.id);

        let mut options: u16 = 0x00;

        options = options
            | match self.packet_type {
                PacketType::Question => 0x00,
                PacketType::Answer => 0x80,
            };

        encode_u16(&mut buf[2..], options);

        // Ignore other fields for now

        // Encode Question Count
        encode_u16(&mut buf[4..], self.question_count);

        // Encode Answer Count
        encode_u16(&mut buf[6..], self.answer_count);

        // Encode Name Server Count
        encode_u16(&mut buf[8..], self.name_server_count);
        // Encode Additional Records Count

        return 12;
    }
}

impl AnswerPacket {
    fn encode(&self, buf: &mut [u8]) -> usize {
        // Encode domain name
        let mut n = encode_domain(&self.domain, buf);

        // Encode type
        let type_bytes: u16 = match self.answer_type {
            QuestionType::ARecord => 0x0001,
            QuestionType::NameServersRecord => 0x0002,
            QuestionType::CNameRecord => 0x0005,
            QuestionType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        encode_u16(&mut buf[n..], type_bytes);

        // Encode class
        encode_u16(&mut buf[n + 2..], 1);

        // Encode time to live
        encode_u16(&mut buf[n + 4..], 0x00);
        encode_u16(&mut buf[n + 6..], self.time_to_live);

        // Encoding RData length field
        encode_u16(&mut buf[n + 8..], 0x04);

        // Encode RDdata field
        encode_u16(&mut buf[n + 10..], 0x0808);
        encode_u16(&mut buf[n + 12..], 0x0808);

        return n + 14;
    }
}

impl QuestionPacket {
    fn encode(&self, buf: &mut [u8]) -> usize {
        // Encode domain name
        let mut n = encode_domain(&self.domain, buf);

        // Encode type
        let type_bytes: u16 = match self.question_type {
            QuestionType::ARecord => 0x0001,
            QuestionType::NameServersRecord => 0x0002,
            QuestionType::CNameRecord => 0x0005,
            QuestionType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        encode_u16(&mut buf[n..], type_bytes);

        // Encode class
        encode_u16(&mut buf[n + 2..], 1);

        return n + 4;
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

struct Frame {}

struct Connection {
    sock: UdpSocket,
    addr: SocketAddr,
    read_buf: NetworkBuffer,
    write_buf: NetworkBuffer,
}

impl Connection {
    pub fn new(sock: UdpSocket, addr: SocketAddr) -> Connection {
        // Initializing buffers
        let read_buf = NetworkBuffer::new();
        let write_buf = NetworkBuffer::new();

        return Connection {
            sock,
            addr,
            read_buf,
            write_buf,
        };
    }

    pub async fn write_frame(&mut self, frame: Frame) {}

    pub async fn read_frame(&mut self, frame: Frame) -> Option<Frame> {
        return Some(Frame {});
    }
}
