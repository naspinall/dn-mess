use crate::errors::NetworkBufferError;
use crate::network_buffer::NetworkBuffer;
use crate::packets::{
    AnswerPacket, HeaderPacket, PacketType, QuestionClass, QuestionPacket, QuestionType,
};

type CodingResult<T> = Result<T, NetworkBufferError>;

pub mod frame_encoder {

    use super::*;

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
        encode_domain(&answer.domain, buf)?;

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
        buf.put_u32(answer.time_to_live)?;

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

pub mod frame_decoder {

    use crate::packets::AnswerData;

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

    pub fn decode_answer(buf: &mut NetworkBuffer) -> CodingResult<AnswerPacket> {
        // Decode the domain
        let domain = decode_domain(buf)?;

        // Decode the type
        let answer_type = match buf.get_u16()? {
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

        let time_to_live = buf.get_u32()?;

        // Assume A record for now
        let answer_data = buf.get_u32()?;

        return Ok(AnswerPacket {
            domain,
            answer_type,
            class,
            time_to_live,
            answer_data: AnswerData::ARecord(answer_data),
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
