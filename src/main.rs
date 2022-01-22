use std::net::SocketAddr;

use tokio::net::UdpSocket;

const HEADER_LENGTH: usize = 12;
const BUFFER_LENGTH: usize = 512;

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
struct HeaderPacket {
    id: u16,
    packet_type: PacketType,
    op_code: u8,

    question_count: u16,
    answer_count: u16,
    name_server_count: u16,
    additional_records_count: u16,
}

#[derive(Debug)]
struct QuestionPacket {
    domain: String,
    question_type: QuestionType,
    class: QuestionClass,
}

#[derive(Debug)]
struct AnswerPacket {
    domain: String,
    answer_type: QuestionType,
    class: QuestionClass,
    time_to_live: u16,
}

struct Buffer {
    current_index: usize,
    buf: [u8; 512],
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            current_index: 0,
            buf: [0; BUFFER_LENGTH],
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), &'static str> {
        // Checking bounds
        if self.current_index == BUFFER_LENGTH {
            return Err("BUFFER FULL");
        }

        // Write the byte
        self.buf[self.current_index] = byte;

        // Increment index
        self.current_index += 1;

        return Ok(());
    }

    fn write_u16(&mut self, value: u16) -> Result<(), &'static str> {
        if self.current_index + 1 == BUFFER_LENGTH {
            return Err("BUFFER FULL");
        }

        self.buf[self.current_index] = (value >> 8) as u8;
        self.buf[self.current_index + 1] = (value & 0x00FF) as u8;

        self.current_index += 2;

        return Ok(());
    }

    fn read_byte(&mut self, byte: u8) -> Result<u8, &'static str> {
        // Checking bounds
        if self.current_index - 1 < 0 {
            return Err("BUFFER EMPTY");
        }

        let byte = self.buf[self.current_index];

        self.current_index -= 1;

        return Ok(byte);
    }

    fn read_u16(&mut self, value: u16) -> Result<u16, &'static str> {
        // Checking bounds
        if self.current_index - 2 < 0 {
            return Err("BUFFER EMPTY");
        }

        let value =
            (self.buf[self.current_index] as u16) << 8 | self.buf[self.current_index + 1] as u16;

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
        let mut buf = [0; 1024];

        let (len, addr) = sock.recv_from(&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);

        println!("{:?} {:?}", buf[0], buf[1]);

        let header = decode_header(&buf);

        let answer_header = HeaderPacket {
            id: header.id,
            packet_type: PacketType::Answer,
            op_code: 0,
            name_server_count: 0,
            question_count: 1,
            answer_count: 1,
            additional_records_count: 0,
        };

        let mut answer_buf = [0; 1024];

        answer_header.encode(&mut answer_buf);

        println!("{:?}", header);

        let question = decode_question(&buf[HEADER_LENGTH..]);

        println!("{:?}", question);

        let m = question.encode(&mut answer_buf[HEADER_LENGTH..]);

        let answer = AnswerPacket {
            domain: question.domain,
            answer_type: QuestionType::ARecord,
            class: QuestionClass::InternetAddress,
            time_to_live: 100,
        };

        println!("{:?}", answer);

        let n = answer.encode(&mut answer_buf[HEADER_LENGTH + m..]);

        println!("{:?}", answer_buf);

        let len = sock
            .send_to(&answer_buf[..HEADER_LENGTH + n + m], addr)
            .await?;
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

struct Connection {
    sock: UdpSocket,
    addr: SocketAddr,
}
