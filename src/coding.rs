use std::collections::HashMap;
use std::vec;

use crate::errors::NetworkBufferError;
use crate::network_buffer::NetworkBuffer;
use crate::packets::{
    Frame, PacketType, Question, QuestionClass, ResourceRecord, ResourceRecordClass,
    ResourceRecordData, ResourceRecordType, ResponseCode, SOARecord,
};

type CodingResult<T> = Result<T, NetworkBufferError>;

pub struct FrameCoder {
    encoded_domains: HashMap<String, usize>,
    decoded_names: HashMap<usize, String>,
}

impl FrameCoder {
    pub fn new() -> FrameCoder {
        FrameCoder {
            decoded_names: HashMap::new(),
            encoded_domains: HashMap::new(),
        }
    }

    pub fn set_compressed_name(&mut self, domain: &str, buf: &NetworkBuffer) {
        // Current index of compression
        let compressed_index = buf.len();

        // Set into hash map
        self.encoded_domains
            .insert(domain.to_string(), compressed_index);
    }

    pub fn get_compressed_name(&self, domain: &str) -> Option<&usize> {
        self.encoded_domains.get(domain)
    }

    pub fn encode_name_label(&mut self, label: &str, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Setting label length
        buf.put_u8(label.len() as u8)?;

        // Add each character
        for character in label.chars() {
            buf.put_u8(character as u8)?;
        }

        Ok(())
    }

    pub fn write_compressed_name(
        &self,
        offset: usize,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        let compressed_offset = 0xC000 | offset as u16;

        buf.put_u16(compressed_offset)
    }

    pub fn encode_name(&mut self, name: &str, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Check if name has already been cached
        if let Some(index) = self.get_compressed_name(name) {
            // Write the whole compressed name
            self.write_compressed_name(*index, buf)?;

            // Will only ever be one byte
            return Ok(());
        };

        // Set domain into hashmap
        self.set_compressed_name(name, buf);

        let labels = name.split('.');

        for label in labels {
            // Skip empty strings
            if label.is_empty() {
                continue;
            }

            // Add length plus one for length byte
            self.encode_name_label(&label.to_string(), buf)?;
        }

        // Terminating name
        buf.put_u8(0x00)?;

        // Return length for null byte
        Ok(())
    }

    pub fn encode_resource_record(
        &mut self,
        resource_record: &ResourceRecord,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_name(&resource_record.domain, buf)?;

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
            ResourceRecordData::CNameRecord(domain) => {
                // Where length should be
                let length_index = buf.write_cursor;

                // Write blank data to where size is
                buf.put_u16(0)?;

                // Write record data, add one for null terminating byte
                let record_data_length = self.encode_resource_record_data(&domain, buf)? + 1;

                // Add null terminating byte
                buf.put_u8(0)?;

                // Set size value
                buf.set_u16(length_index, record_data_length as u16)?;

                Ok(())
            }
            ResourceRecordData::SOARecord(record) => self.encode_soa_record(record, buf),
        }
    }

    pub fn encode_resource_record_data(
        &mut self,
        domain: &str,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<usize> {
        let starting_index = buf.write_cursor;

        // Check if domain has already been cached
        if let Some(index) = self.get_compressed_name(domain) {
            // Write the whole compressed domain
            self.write_compressed_name(*index, buf)?;

            return Ok(buf.write_cursor - starting_index);
        };

        // Set domain into hashmap
        self.set_compressed_name(domain, buf);

        let labels = domain.split('.');

        for label in labels {
            // Skip empty strings
            if label.is_empty() {
                continue;
            }

            // Add length plus one for length byte
            self.encode_name_label(&label.to_string(), buf)?;
        }

        // Return length for null byte
        Ok(buf.write_cursor - starting_index)
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
        self.encode_name(&question.domain, buf)?;

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

    pub fn encode_soa_record(
        &mut self,
        soa_record: &SOARecord,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_name(&soa_record.master_name, buf)?;
        self.encode_name(&soa_record.mail_name, buf)?;

        buf.put_u32(soa_record.serial)?;
        buf.put_u32(soa_record.refresh)?;
        buf.put_u32(soa_record.retry)?;
        buf.put_u32(soa_record.expire)?;
        buf.put_u32(soa_record.minimum)?;

        Ok(())
    }

    pub fn decode_question(&mut self, buf: &mut NetworkBuffer) -> CodingResult<Question> {
        // Decode the domain
        let domain = self.decode_name(buf)?;

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

    pub fn decode_name_label(
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

    pub fn save_decoded_names(&mut self, domains: &[String], indexes: &[usize]) {
        // Adding all domains decoded, indexes in reverse order
        for (domain_index, pointer_index) in (0..domains.len()).zip(indexes.iter()) {
            // Join all the domains from domain_index, as that corresponds to all labels after the pointer index
            let full_domain = domains[domain_index..].join(".");
            self.decoded_names.insert(*pointer_index, full_domain);
        }
    }

    pub fn get_pointer_location(&self, left: u8, right: u8) -> usize {
        (((0x3F & left) as u16) << 8 | right as u16) as usize
    }

    pub fn decode_name(&mut self, buf: &mut NetworkBuffer) -> CodingResult<String> {
        // Keep track of the index, so we can cache any pointers
        let mut starting_index = buf.read_cursor;
        let mut label_length = buf.get_u8()? as usize;

        let mut decoded_names = vec![];
        let mut decoded_indexes = vec![];

        while label_length != 0x00 {
            // Check for a pointer to existing labels
            if label_length & 0xC0 > 0 {
                // Get the location of the pointer
                let pointer_location = self.get_pointer_location(label_length as u8, buf.get_u8()?);

                // Get from the cached values
                let name = match self.decoded_names.get(&pointer_location) {
                    Some(name) => name.clone(),
                    None => return Err(NetworkBufferError::CompressionError),
                };

                // Add to list of domains labels we have parsed
                decoded_names.push(name);

                // Pointer means we are done, so exit here
                break;
            }

            // Decode current label
            let label = self.decode_name_label(label_length, buf)?;

            // Add to list of decoded domains
            decoded_names.push(label);
            decoded_indexes.push(starting_index);

            // Setup for the next label
            starting_index = buf.read_cursor;
            label_length = buf.get_u8()? as usize;
        }

        self.save_decoded_names(&decoded_names, &decoded_indexes);

        let mut name = decoded_names.join(".");

        name.push('.');

        Ok(name)
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
        let domain = self.decode_name(buf)?;
        let record_type = self.decode_type(buf)?;
        let class = self.decode_class(buf)?;
        let time_to_live = buf.get_u32()?;

        // TODO verify data length here
        let _data_length = buf.get_u16()?;

        let record_data = match record_type {
            ResourceRecordType::ARecord => ResourceRecordData::ARecord(buf.get_u32()?),
            ResourceRecordType::CNameRecord => {
                ResourceRecordData::CNameRecord(self.decode_name(buf)?)
            }
            ResourceRecordType::AAAARecord => ResourceRecordData::AAAARecord(buf.get_u128()?),
            ResourceRecordType::SOARecord => {
                ResourceRecordData::SOARecord(self.decode_soa_record(buf)?)
            }
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

    pub fn decode_soa_record(&mut self, buf: &mut NetworkBuffer) -> CodingResult<SOARecord> {
        Ok(SOARecord {
            master_name: self.decode_name(buf)?,
            mail_name: self.decode_name(buf)?,
            serial: buf.get_u32()?,
            refresh: buf.get_u32()?,
            retry: buf.get_u32()?,
            expire: buf.get_u32()?,
            minimum: buf.get_u32()?,
        })
    }

    pub fn encode_frame(&mut self, frame: &Frame, buf: &mut NetworkBuffer) -> CodingResult<()> {
        self.encode_header(&frame, buf)?;

        // Encode question
        for question in frame.questions.iter() {
            self.encode_question(question, buf)?;
        }

        // Encode answers
        for answer in frame.answers.iter() {
            self.encode_resource_record(answer, buf)?;
        }

        // Encode name servers
        for name_server in frame.name_servers.iter() {
            self.encode_resource_record(name_server, buf)?;
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
        let name_server_count = buf.get_u16()?;
        let _additional_records_count = buf.get_u16()?;

        let mut questions: Vec<Question> = Vec::new();
        let mut answers: Vec<ResourceRecord> = Vec::new();
        let mut name_servers: Vec<ResourceRecord> = Vec::new();

        // Encode question
        for _ in 0..question_count {
            let question = self.decode_question(buf)?;
            questions.push(question);
        }

        // Encode answers
        for _ in 0..answer_count {
            let answer = self.decode_resource_record(buf)?;
            answers.push(answer);
        }

        // Encode name server
        for _ in 0..name_server_count {
            let name_server = self.decode_resource_record(buf)?;
            name_servers.push(name_server);
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
            name_servers,
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

        let domain = coder.decode_name(&mut buf).unwrap();

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

        let domain = coder.decode_name(&mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from(".hello.com"));
    }

    #[test]
    fn test_decode_label() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 5] = [b'h', b'e', b'l', b'l', b'o'];

        buf._put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_name_label(5, &mut buf).unwrap();

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

        let resource_record_bytes = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 5, 0, 1, 0,
            0, 0, 255, 0, 15, 3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109,
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
            ResourceRecordData::CNameRecord(value) => assert_eq!(value, ".www.google.com"),
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

        let original = coder.decode_name(&mut buf).unwrap();

        assert_eq!(original, String::from(".www.google.com"));

        let pointer = coder.decode_name(&mut buf).unwrap();

        assert_eq!(original, pointer);
    }

    #[test]
    fn test_decode_double_pointer_cname_request() {
        let mut buf = NetworkBuffer::new();

        buf._put_bytes(&[
            5, 100, 128, 128, 0, 1, 0, 2, 0, 0, 0, 0, 3, 119, 119, 119, 8, 102, 97, 99, 101, 98,
            111, 111, 107, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 9, 125, 0,
            17, 9, 115, 116, 97, 114, 45, 109, 105, 110, 105, 4, 99, 49, 48, 114, 192, 16, 192, 46,
            0, 1, 0, 1, 0, 0, 0, 14, 0, 4, 157, 240, 18, 35, 0,
        ])
        .unwrap();

        let mut coder = FrameCoder::new();

        let frame = coder.decode_frame(&mut buf).unwrap();

        assert!(frame.answers.len() > 0);
        assert_eq!(frame.answers[0].domain, ".www.facebook.com");
        assert_eq!(
            frame.answers[0].record_type,
            ResourceRecordType::CNameRecord
        );
        assert_eq!(
            frame.answers[0].data,
            ResourceRecordData::CNameRecord(".star-mini.c10r.facebook.com".to_string()),
        );
    }
}
