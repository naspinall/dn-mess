use std::collections::HashMap;

use crate::errors::NetworkBufferError;
use crate::network_buffer::NetworkBuffer;
use crate::packets::{
    HeaderPacket, PacketType, QuestionClass, QuestionPacket, QuestionType, ResourceRecordClass,
    ResourceRecordData, ResourceRecordPacket, ResourceRecordType,
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

    pub fn set_compressed_domain(&mut self, domain: &String, buf: &NetworkBuffer) {
        // Current index of compression
        let compressed_index = buf.len();

        // Set into hash map
        self.encoded_domains
            .insert(domain.clone(), compressed_index);
    }

    pub fn get_compressed_domain(&self, domain: &String) -> Option<&usize> {
        self.encoded_domains.get(domain)
    }

    pub fn encode_domain_label(
        &mut self,
        label: &String,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Setting label length
        buf.put_u8(label.len() as u8)?;

        // Add each character
        for character in label.chars() {
            buf.put_u8(character as u8)?;
        }

        Ok(())
    }

    pub fn write_compressed_domain(
        &self,
        offset: usize,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        let compressed_offset = 0xC000 | offset as u16;

        return buf.put_u16(compressed_offset);
    }

    pub fn encode_domain(&mut self, domain: &String, buf: &mut NetworkBuffer) -> CodingResult<()> {
        // Check if domain has already been cached
        match self.get_compressed_domain(&domain) {
            // Write the compressed full domain and return
            Some(index) => return self.write_compressed_domain(*index, buf),
            // Do nothing if domain has not been compressed
            None => {}
        };

        // Set domain into hashmap
        self.set_compressed_domain(domain, &buf);

        let labels = domain.split(".");

        for label in labels {
            // Skip empty strings
            if label.is_empty() {
                continue;
            }

            self.encode_domain_label(&label.to_string(), buf)?;
        }

        // Terminating domain name
        buf.put_u8(0x00)
    }

    pub fn encode_answer(
        &mut self,
        answer: &ResourceRecordPacket,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_domain(&answer.domain, buf)?;

        // Encode type
        let type_bytes: u16 = match answer.record_type {
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
        buf.put_u32(answer.time_to_live)?;

        // Encoding RData length field
        buf.put_u16(0x04)?;

        // Encode RDdata field
        buf.put_u16(0x0808)?;
        buf.put_u16(0x0808)
    }

    pub fn encode_header(
        &mut self,
        header: &HeaderPacket,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
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

    pub fn encode_question(
        &mut self,
        question: &QuestionPacket,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<()> {
        // Encode domain name
        self.encode_domain(&question.domain, buf)?;

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

    pub fn decode_header(&mut self, buf: &mut NetworkBuffer) -> CodingResult<HeaderPacket> {
        // decode ID field
        let id = buf.get_u16()?;

        // decode query response bit

        let flag_byte = buf.get_u8()?;

        let packet_type = match 0x01 & flag_byte == 1 {
            true => PacketType::Question,
            false => PacketType::Answer,
        };

        let op_code = (flag_byte >> 1) as u8 & 0x0F;
        let authoritative_answer = flag_byte >> 5 & 0x01 == 1;
        let truncation = flag_byte >> 6 & 0x01 == 1;
        let recursion_desired = flag_byte >> 7 & 0x01 == 1;

        let flag_byte = buf.get_u8()?;

        let recursion_available = flag_byte & 0x01 == 1;
        let response_code = flag_byte >> 4 & 0x0F;

        let question_count = buf.get_u16()?;
        let answer_count = buf.get_u16()?;
        let name_server_count = buf.get_u16()?;
        let additional_records_count = buf.get_u16()?;

        return Ok(HeaderPacket {
            id,
            packet_type,
            op_code,

            authoritative_answer,
            truncation,
            recursion_desired,
            recursion_available,
            response_code,

            question_count,
            answer_count,
            name_server_count,
            additional_records_count,
        });
    }

    pub fn decode_question(&mut self, buf: &mut NetworkBuffer) -> CodingResult<QuestionPacket> {
        // Decode the domain
        let domain = self.decode_domain(buf)?;

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

    pub fn decode_domain_label(
        &mut self,
        length: usize,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<String> {
        let mut label = String::new();

        let mut n = 0;

        while n < length {
            label.push(buf.get_u8()? as char);
            n = n + 1;
        }

        return Ok(label);
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

        return Ok(domain);
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

        return Ok(class);
    }

    pub fn decode_resource_record(
        &mut self,
        buf: &mut NetworkBuffer,
    ) -> CodingResult<ResourceRecordPacket> {
        // Decoding domain name record refers too
        let domain = self.decode_domain(buf)?;
        let record_type = self.decode_type(buf)?;
        let class = self.decode_class(buf)?;
        let time_to_live = buf.get_u32()?;
        let data_length = buf.get_u16()?;
        let payload = buf.get_u32()?;

        return Ok(ResourceRecordPacket {
            domain,
            record_type,
            record_data: ResourceRecordData::ARecord(payload),
            class,
            time_to_live,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_domain() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 7] = [
            0x05, 'h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8, 0x00,
        ];

        buf.put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain(&mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from(".hello"));
    }

    #[test]
    fn test_decode_domain() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 11] = [
            0x05, 'h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8, 0x03, 'c' as u8,
            'o' as u8, 'm' as u8, 0x00,
        ];

        buf.put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain(&mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from(".hello.com"));
    }

    #[test]
    fn test_decode_label() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let domain_bytes: [u8; 5] = ['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8];

        buf.put_bytes(&domain_bytes).unwrap();

        let domain = coder.decode_domain_label(5, &mut buf).unwrap();

        // A . is appended so include here
        assert_eq!(domain, String::from("hello"));
    }

    #[test]
    fn test_decode_header() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let header_bytes: [u8; 12] = [112, 181, 0xE5, 0x41, 0, 1, 0, 2, 0, 3, 0xFF, 0x11];

        buf.put_bytes(&header_bytes).unwrap();

        let header = coder.decode_header(&mut buf).unwrap();

        assert_eq!(header.id, 28853);
        assert_eq!(header.op_code, 0x02);
        assert!(match header.packet_type {
            PacketType::Question => true,
            _ => false,
        });

        assert_eq!(header.authoritative_answer, true);
        assert_eq!(header.truncation, true);
        assert_eq!(header.recursion_desired, true);
        assert_eq!(header.recursion_available, true);
        assert_eq!(header.response_code, 4);

        assert_eq!(header.question_count, 1);
        assert_eq!(header.answer_count, 2);
        assert_eq!(header.name_server_count, 3);
        assert_eq!(header.additional_records_count, 65297);
    }

    #[test]
    fn test_decode_question() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let question_bytes: [u8; 20] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1,
        ];

        buf.put_bytes(&question_bytes).unwrap();

        let question = coder.decode_question(&mut buf).unwrap();

        assert_eq!(question.domain, String::from(".www.google.com"));
        assert!(match question.question_type {
            QuestionType::ARecord => true,
            _ => false,
        });

        assert!(match question.class {
            QuestionClass::InternetAddress => true,
            _ => false,
        })
    }

    #[test]
    fn test_decode_resource_record() {
        let mut coder = FrameCoder::new();
        let mut buf = NetworkBuffer::new();

        let resource_record_bytes: [u8; 30] = [
            3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 0,
            0, 0, 255, 0, 4, 8, 8, 8, 8,
        ];

        buf.put_bytes(&resource_record_bytes).unwrap();

        let resource_record = coder.decode_resource_record(&mut buf).unwrap();

        assert_eq!(resource_record.domain, String::from(".www.google.com"));
        assert!(match resource_record.record_type {
            ResourceRecordType::ARecord => true,
            _ => false,
        });

        assert!(match resource_record.class {
            ResourceRecordClass::InternetAddress => true,
            _ => false,
        });

        assert_eq!(resource_record.time_to_live, 255);
        match resource_record.record_data {
            ResourceRecordData::ARecord(value) => assert_eq!(value, 0x08080808),
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

        buf.put_bytes(&pointer_domain_bytes).unwrap();

        let original = coder.decode_domain(&mut buf).unwrap();

        assert_eq!(original, String::from(".www.google.com"));

        let pointer = coder.decode_domain(&mut buf).unwrap();

        assert_eq!(original, pointer);
    }
}
