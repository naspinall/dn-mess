use std::collections::HashMap;

use crate::errors::NetworkBufferError;
use crate::network_buffer::NetworkBuffer;
use crate::packets::{
    Frame, PacketType, Question, QuestionClass, ResourceRecord, ResourceRecordClass,
    ResourceRecordData, ResourceRecordType, ResponseCode,
};

type CodingResult<T> = Result<T, NetworkBufferError>;

pub struct FrameCoder {
    encoded_domains: HashMap<String, usize>,
    decoded_domains: HashMap<usize, String>,
}

impl FrameCoder {
    pub fn new() -> FrameCoder {
        FrameCoder {
            decoded_domains: HashMap::new(),
            encoded_domains: HashMap::new(),
        }
    }

    pub fn set_compressed_domain(&mut self, domain: &str, buf: &NetworkBuffer) {
        // Current index of compression
        let compressed_index = buf.len();

        // Set into hash map
        self.encoded_domains
            .insert(domain.to_string(), compressed_index);
    }

    pub fn get_compressed_domain(&self, domain: &str) -> Option<&usize> {
        self.encoded_domains.get(domain)
    }

    pub fn encode_domain_label(
        &mut self,
        label: &str,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<usize> {
        // Setting label length
        buf.put_u8(label.len() as u8)?;

        // Add each character
        for character in label.chars() {
            buf.put_u8(character as u8)?;
        }

        Ok(label.len())
    }

    pub fn write_compressed_domain(
        &self,
        offset: usize,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        let compressed_offset = 0xC000 | offset as u16;

        buf.put_u16(compressed_offset)
    }

    fn calculate_domain_length(domain: &str) -> usize {
        let mut total_length = 0;

        let labels = domain.split(".");

        labels
            .into_iter()
            .for_each(|label| total_length += label.len() + 1);

        total_length
    }

    pub fn encode_domain(&mut self, domain: &str, buf: &mut NetworkBuffer) -> CodingResult<usize> {
        let mut encoded_length = 0;

        // Check if domain has already been cached
        if let Some(index) = self.get_compressed_domain(domain) {
            // Write the whole compressed domain
            self.write_compressed_domain(*index, buf)?;

            // Will only ever be one byte
            return Ok(1);
        };

        // Set domain into hashmap
        self.set_compressed_domain(domain, buf);

        let labels = domain.split('.');

        for label in labels {
            // Skip empty strings
            if label.is_empty() {
                continue;
            }

            // Add length plus one for length byte
            encoded_length += self.encode_domain_label(&label.to_string(), buf)? + 1;
        }

        // Terminating domain name
        buf.put_u8(0x00)?;

        // Add for null byte
        encoded_length += 1;

        // Return length for null byte
        Ok(encoded_length)
    }

    pub fn encode_resource_record(
        &mut self,
        resource_record: &ResourceRecord,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_domain(&resource_record.domain, buf)?;

        // Encode type
        let type_bytes: u16 = match resource_record.record_type {
            ResourceRecordType::ARecord => 0x0001,
            ResourceRecordType::NSRecord => 0x0002,
            ResourceRecordType::CNameRecord => 0x0005,
            ResourceRecordType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        buf.put_u16(type_bytes)?;

        // Encode class
        buf.put_u16(1)?;

        // Encode time to live
        buf.put_u32(resource_record.time_to_live)?;

        // Encode RDdata field
        match &resource_record.data {
            ResourceRecordData::ARecord(record) => {
                buf.put_u16(4)?;
                buf.put_u32(*record)
            }
            ResourceRecordData::AAAARecord(record) => {
                buf.put_u16(16)?;
                buf.put_u128(*record)
            }
            ResourceRecordData::CName(domain) => {
                let rd_length = FrameCoder::calculate_domain_length(&domain);

                buf.put_u16(0)?;
                let encoded_length = self.encode_domain(&domain, buf)?;

                if encoded_length != rd_length {
                    return Err(NetworkBufferError::InvalidPacket);
                }

                Ok(())
            }
        }
    }

    pub fn encode_header(&mut self, frame: &Frame, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Encode the packet ID
        buf.put_u16(frame.id)?;

        let mut options: u8 = 0x00;

        options |= match frame.packet_type {
            PacketType::Query => 0x00,
            PacketType::Response => 0x80,
        };

        options |= (frame.op_code & 0x0F) << 3;
        options |= if frame.authoritative_answer {
            0x04
        } else {
            0x00
        };
        options |= if frame.truncation { 0x02 } else { 0x00 };
        options |= if frame.recursion_desired { 0x01 } else { 0x00 };

        buf.put_u8(options)?;

        options = 0x00;

        options |= if frame.recursion_available { 0x80 } else { 0x0 };

        options |= match frame.response_code {
            ResponseCode::None => 0,
            ResponseCode::FormatError => 1,
            ResponseCode::ServerError => 2,
            ResponseCode::NameError => 3,
            ResponseCode::NotImplemented => 4,
            ResponseCode::Refused => 5,
        } & 0x0F;

        buf.put_u8(options)?;

        // Encode Question Count
        buf.put_u16(frame.questions.len() as u16)?;

        // Encode Answer Count
        buf.put_u16(frame.answers.len() as u16)?;

        // Encode Name Server Count
        buf.put_u16(frame.name_servers.len() as u16)?;
        // Encode Additional Records Count

        buf.put_u16(frame.additional_records.len() as u16)
    }

    pub fn encode_question(
        &mut self,
        question: &Question,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_domain(&question.domain, buf)?;

        // Encode type
        let type_bytes: u16 = match question.question_type {
            ResourceRecordType::ARecord => 0x0001,
            ResourceRecordType::NSRecord => 0x0002,
            ResourceRecordType::CNameRecord => 0x0005,
            ResourceRecordType::MXRecord => 0x000f,
            _ => 0x0000,
        };

        // Encode the type
        buf.put_u16(type_bytes)?;

        // Encode class
        buf.put_u16(1)
    }

    pub fn decode_question(&mut self, buf: &mut NetworkBuffer) -> CodingResult<Question> {
        // Decode the domain
        let domain = self.decode_domain(buf)?;

        // Decode the type
        let question_type = match buf.get_u16()? {
            0x0001 => ResourceRecordType::ARecord,
            0x0002 => ResourceRecordType::NSRecord,
            0x0005 => ResourceRecordType::CNameRecord,
            0x000f => ResourceRecordType::MXRecord,
            _ => ResourceRecordType::Unimplemented,
        };

        // Decode the class
        let class = match buf.get_u16()? {
            0x001 => QuestionClass::InternetAddress,
            _ => QuestionClass::Unimplemented,
        };

        Ok(Question {
            domain,
            question_type,
            class,
        })
    }

    pub fn decode_domain_label(
        &mut self,
        length: usize,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<String> {
        let mut label = String::new();

        let mut n = 0;

        while n < length {
            label.push(buf.get_u8()? as char);
            n += 1;
        }

        Ok(label)
    }

    pub fn decode_domain(&mut self, buf: &mut NetworkBuffer) -> CodingResult<String> {
        let mut domain = String::new();

        let starting_index = buf.read_cursor;

        let mut label_length = buf.get_u8()? as usize;

        while label_length != 0x00 {
            if label_length & 0xC0 > 0 {
                // TODO two byte offset
                let pointer_location = buf.get_u8()?;

                return Ok(self
                    .decoded_domains
                    .get(&(pointer_location as usize))
                    .unwrap()
                    .to_string());
            }

            // Decode current label
            let label = self.decode_domain_label(label_length, buf)?;

            // Add separator
            domain.push('.');

            // Add the label to the total domain
            domain.push_str(&label);

            label_length = buf.get_u8()? as usize;
        }

        // Add to cache
        self.decoded_domains.insert(starting_index, domain.clone());

        Ok(domain)
    }

    pub fn decode_type(&mut self, buf: &mut NetworkBuffer) -> CodingResult<ResourceRecordType> {
        let record_type = match buf.get_u16()? {
            0x01 => ResourceRecordType::ARecord,
            0x1C => ResourceRecordType::AAAARecord,
            0x05 => ResourceRecordType::CNameRecord,
            0x0F => ResourceRecordType::MXRecord,
            0x02 => ResourceRecordType::NSRecord,
            0x0C => ResourceRecordType::PTRRecord,
            0x06 => ResourceRecordType::SOARecord,
            0x21 => ResourceRecordType::SRVRecord,
            0x10 => ResourceRecordType::TXTRecord,
            _ => ResourceRecordType::Unimplemented,
        };

        Ok(record_type)
    }

    pub fn decode_class(&mut self, buf: &mut NetworkBuffer) -> CodingResult<ResourceRecordClass> {
        let class = match buf.get_u16()? {
            0x001 => ResourceRecordClass::InternetAddress,
            _ => ResourceRecordClass::Unimplemented,
        };

        Ok(class)
    }

    pub fn decode_resource_record(
        &mut self,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<ResourceRecord> {
        // Decoding domain name record refers too
        let domain = self.decode_domain(buf)?;
        let record_type = self.decode_type(buf)?;
        let class = self.decode_class(buf)?;
        let time_to_live = buf.get_u32()?;

        // TODO verify data length here
        let _data_length = buf.get_u16()?;

        let record_data = match record_type {
            ResourceRecordType::ARecord => ResourceRecordData::ARecord(buf.get_u32()?),
            ResourceRecordType::CNameRecord => ResourceRecordData::CName(self.decode_domain(buf)?),
            ResourceRecordType::AAAARecord => ResourceRecordData::AAAARecord(buf.get_u128()?),
            _ => return Err(NetworkBufferError::InvalidPacket),
        };

        Ok(ResourceRecord {
            domain,
            record_type,
            data: record_data,
            class,
            time_to_live,
        })
    }

    pub fn encode_frame(&mut self, frame: &Frame, buf: &mut NetworkBuffer) -> CodingResult<()> {
        self.encode_header(&frame, buf)?;

        // Encode question
        for question in frame.questions.iter() {
            self.encode_question(question, buf)?;
        }

        // Encode question
        for answer in frame.answers.iter() {
            self.encode_resource_record(answer, buf)?;
        }

        Ok(())
    }

    pub fn decode_frame(&mut self, buf: &mut NetworkBuffer) -> CodingResult<Frame> {
        // decode ID field
        let id = buf.get_u16()?;

        // decode query response bit

        let flag_byte = buf.get_u8()?;

        let packet_type = match 0x80 & flag_byte == 0x80 {
            true => PacketType::Response,
            false => PacketType::Query,
        };

        let op_code = (flag_byte >> 3) as u8 & 0x0F;
        let authoritative_answer = flag_byte >> 2 & 0x01 == 1;
        let truncation = flag_byte >> 1 & 0x01 == 1;
        let recursion_desired = flag_byte & 0x01 == 1;

        let flag_byte = buf.get_u8()?;

        let recursion_available = flag_byte >> 7 & 0x01 == 1;
        let response_code = match flag_byte & 0x0F {
            0 => ResponseCode::None,
            1 => ResponseCode::FormatError,
            2 => ResponseCode::ServerError,
            3 => ResponseCode::NameError,
            4 => ResponseCode::NotImplemented,
            5 => ResponseCode::Refused,
            _ => return Err(NetworkBufferError::InvalidPacket),
        };

        let question_count = buf.get_u16()?;
        let answer_count = buf.get_u16()?;
        let _name_server_count = buf.get_u16()?;
        let _additional_records_count = buf.get_u16()?;

        let mut questions: Vec<Question> = Vec::new();
        let mut answers: Vec<ResourceRecord> = Vec::new();

        // Encode question
        for _ in 0..question_count {
            let question = self.decode_question(buf)?;
            questions.push(question);
        }

        // Encode question
        for _ in 0..answer_count {
            let answer = self.decode_resource_record(buf)?;
            answers.push(answer);
        }

        Ok(Frame {
            id,
            packet_type,
            op_code,

            authoritative_answer,
            truncation,
            recursion_desired,
            recursion_available,
            response_code,

            questions,
            answers,
            name_servers: vec![],
            additional_records: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_domain() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 7] = [0x05, b'h', b'e', b'l', b'l', b'o', 0x00];

        buf._put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain(&mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from(".hello"));
    }

    #[test]
    fn test_decode_domain() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 11] = [
            0x05, b'h', b'e', b'l', b'l', b'o', 0x03, b'c', b'o', b'm', 0x00,
        ];

        buf._put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain(&mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from(".hello.com"));
    }

    #[test]
    fn test_decode_label() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 5] = [b'h', b'e', b'l', b'l', b'o'];

        buf._put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain_label(5, &mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from("hello"));
    }

    #[test]
    fn test_decode_header() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let header_bytes: [u8; 12] = [112, 181, 151, 132, 0, 0, 0, 0, 0, 0, 0, 0];

        buf._put_bytes(&header_bytes).unwrap();

        let frame = coder.decode_frame(&mut buf).unwrap();

        assert_eq!(frame.id, 28853);
        assert_eq!(frame.op_code, 0x02);
        assert!(matches!(frame.packet_type, PacketType::Response));

        assert!(frame.authoritative_answer);
        assert!(frame.truncation);
        assert!(frame.recursion_desired);
        assert!(frame.recursion_available);
        assert!(matches!(frame.response_code, ResponseCode::NotImplemented));
    }

    #[test]
    fn test_encode_frame() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let frame = Frame {
            id: 28853,
            op_code: 0x02,
            packet_type: PacketType::Response,
            authoritative_answer: true,
            truncation: true,
            recursion_desired: true,
            recursion_available: true,
            response_code: ResponseCode::NotImplemented,
            questions: vec![],
            answers: vec![],
            additional_records: vec![],
            name_servers: vec![],
        };

        coder.encode_frame(&frame, &mut buf).unwrap();

        let expected_bytes: [u8; 12] = [112, 181, 151, 132, 0, 0, 0, 0, 0, 0, 0, 0];

        assert_eq!(expected_bytes, buf.buf[..12]);
    }

    #[test]
    fn test_decode_question() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let question_bytes: [u8; 20] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1,
        ];

        buf._put_bytes(&question_bytes).unwrap();

        let question = coder.decode_question(&mut buf).unwrap();

        assert_eq!(question.domain, String::from(".www.google.com"));
        assert!(matches!(
            question.question_type,
            ResourceRecordType::ARecord
        ));

        assert!(matches!(question.class, QuestionClass::InternetAddress))
    }

    #[test]
    fn test_decode_resource_record() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let resource_record_bytes: [u8; 30] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 0,
            0, 0, 255, 0, 4, 8, 8, 8, 8,
        ];

        buf._put_bytes(&resource_record_bytes).unwrap();

        let resource_record = coder.decode_resource_record(&mut buf).unwrap();

        assert_eq!(resource_record.domain, String::from(".www.google.com"));
        assert!(matches!(
            resource_record.record_type,
            ResourceRecordType::ARecord
        ));

        assert!(matches!(
            resource_record.class,
            ResourceRecordClass::InternetAddress
        ));

        assert_eq!(resource_record.time_to_live, 255);
        match resource_record.data {
            ResourceRecordData::ARecord(value) => assert_eq!(value, 0x08080808),
            _ => panic!("Bad resource record"),
        }
    }

    #[test]
    fn test_decode_resource_record_AAARecord() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let resource_record_bytes: [u8; 42] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 28, 0, 1, 0,
            0, 0, 255, 0, 16, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        ];

        buf._put_bytes(&resource_record_bytes).unwrap();

        let resource_record = coder.decode_resource_record(&mut buf).unwrap();

        assert_eq!(resource_record.domain, String::from(".www.google.com"));
        assert!(matches!(
            resource_record.record_type,
            ResourceRecordType::AAAARecord
        ));

        assert!(matches!(
            resource_record.class,
            ResourceRecordClass::InternetAddress
        ));

        assert_eq!(resource_record.time_to_live, 255);
        match resource_record.data {
            ResourceRecordData::AAAARecord(value) => {
                assert_eq!(value, 0x08080808080808080808080808080808)
            }
            _ => panic!("Bad resource record"),
        }
    }

    #[test]
    fn test_decode_resource_record_CNAME() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let resource_record_bytes: [u8; 42] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 5, 0, 1, 0,
            0, 0, 255, 0, 16, 3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109,
            0,
        ];

        buf._put_bytes(&resource_record_bytes).unwrap();

        let resource_record = coder.decode_resource_record(&mut buf).unwrap();

        assert_eq!(resource_record.domain, String::from(".www.google.com"));
        assert!(matches!(
            resource_record.record_type,
            ResourceRecordType::CNameRecord
        ));

        assert!(matches!(
            resource_record.class,
            ResourceRecordClass::InternetAddress
        ));

        assert_eq!(resource_record.time_to_live, 255);
        match resource_record.data {
            ResourceRecordData::CName(value) => assert_eq!(value, ".www.google.com"),
            _ => panic!("Bad resource record"),
        }
    }

    #[test]
    fn test_decode_pointer_domain() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let pointer_domain_bytes: [u8; 18] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 192, 0,
        ];

        buf._put_bytes(&pointer_domain_bytes).unwrap();

        let original = coder.decode_domain(&mut buf).unwrap();

        assert_eq!(original, String::from(".www.google.com"));

        let pointer = coder.decode_domain(&mut buf).unwrap();

        assert_eq!(original, pointer);
    }

    #[test]
    fn test_calculate_domain_length() {
        let domain = ".www.google.com";

        assert_eq!(FrameCoder::calculate_domain_length(domain), 16);

        let domain = ".dank.com";

        assert_eq!(FrameCoder::calculate_domain_length(domain), 10);
    }
}
