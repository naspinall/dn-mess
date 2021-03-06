use std::{
    fmt,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Debug, Clone)]
pub enum PacketType {
    Query,
    Response,
}

#[derive(Debug, Clone)]

pub enum QuestionClass {
    InternetAddress,
    Unimplemented,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceRecordType {
    ARecord,
    AAAARecord,
    CNameRecord,
    MXRecord,
    NSRecord,
    PTRRecord,
    SOARecord,
    SRVRecord,
    TXTRecord,
    Unimplemented,
}

#[derive(Debug, Clone)]
pub enum ResourceRecordClass {
    InternetAddress,
    Unimplemented,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceRecordData {
    A(u32),
    AAAA(u128),
    CName(String),
    SOA(SOARecord),
    MX(u16, String),
    TXT(String),
    NS(String),
}

impl ResourceRecordData {
    // This shouldn't need to exist, should just store the type in the data
    pub fn get_type(&self) -> ResourceRecordType {
        match self {
            ResourceRecordData::A(_) => ResourceRecordType::ARecord,
            ResourceRecordData::AAAA(_) => ResourceRecordType::AAAARecord,
            ResourceRecordData::CName(_) => ResourceRecordType::CNameRecord,
            ResourceRecordData::SOA(_) => ResourceRecordType::SOARecord,
            ResourceRecordData::MX(_, _) => ResourceRecordType::MXRecord,
            ResourceRecordData::NS(_) => ResourceRecordType::NSRecord,
            ResourceRecordData::TXT(_) => ResourceRecordType::TXTRecord,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResponseCode {
    None,
    FormatError,
    ServerError,
    NameError,
    NotImplemented,
    Refused,
}

#[derive(Debug, Clone)]
pub struct Question {
    pub domain: String,
    pub question_type: ResourceRecordType,
    pub class: QuestionClass,
}

#[derive(Debug, Clone)]
pub struct ResourceRecord {
    pub domain: String,
    pub record_type: ResourceRecordType,
    pub class: ResourceRecordClass,
    pub time_to_live: u32,
    pub data: ResourceRecordData,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SOARecord {
    pub master_name: String,
    pub mail_name: String,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u16,
    pub packet_type: PacketType,
    pub op_code: u8,
    pub authoritative_answer: bool,
    pub truncation: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub response_code: ResponseCode,

    pub questions: Vec<Question>,
    pub answers: Vec<ResourceRecord>,
    pub authorities: Vec<ResourceRecord>,
    pub additional_records: Vec<ResourceRecord>,
}

impl Message {
    /// Get a record from answers first or additional records second
    pub fn get_record(
        &self,
        record_type: &ResourceRecordType,
        domain: &str,
    ) -> Option<&ResourceRecord> {
        // Find A record in answers if present
        let record = self
            .answers
            .iter()
            .find(|record| record.record_type.eq(record_type) && record.domain.eq(domain));

        // Return if found an A record
        if record.is_some() {
            return record;
        }

        let record = self
            .authorities
            .iter()
            .find(|record| record.record_type.eq(record_type) && record.domain.eq(domain));

        // Return if found an A record
        if record.is_some() {
            return record;
        }

        // Check additional records after for an A record
        self.additional_records
            .iter()
            .find(|record| record.record_type.eq(record_type) && record.domain.eq(domain))
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id:{} type:{} ", self.id, self.packet_type)?;

        if !self.questions.is_empty() {
            for question in self.questions.iter() {
                write!(f, "question -> {}", question)?;
            }
        }
        if !self.answers.is_empty() {
            for answer in self.answers.iter() {
                write!(f, "answer -> {}", answer)?;
            }
        }

        if !self.authorities.is_empty() {
            for authority in self.authorities.iter() {
                write!(f, "authority -> {}", authority)?;
            }
        }

        if !self.authorities.is_empty() {
            for additional in self.additional_records.iter() {
                write!(f, "additional -> {}", additional)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for PacketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketType::Query => write!(f, "query"),
            PacketType::Response => write!(f, "response"),
        }
    }
}

impl fmt::Display for ResourceRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceRecordType::ARecord => write!(f, "ARecord"),
            ResourceRecordType::AAAARecord => write!(f, "AAAARecord"),
            ResourceRecordType::CNameRecord => write!(f, "CNameRecord"),
            ResourceRecordType::MXRecord => write!(f, "MXRecord"),
            ResourceRecordType::NSRecord => write!(f, "NSRecord"),
            ResourceRecordType::PTRRecord => write!(f, "PTRRecord"),
            ResourceRecordType::SOARecord => write!(f, "SOARecord"),
            ResourceRecordType::SRVRecord => write!(f, "SRVRecord"),
            ResourceRecordType::TXTRecord => write!(f, "TXTRecord"),
            ResourceRecordType::Unimplemented => write!(f, "Unimplemented"),
        }
    }
}

impl fmt::Display for Question {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "domain:{} type:{} ", self.domain, self.question_type)
    }
}

impl fmt::Display for ResourceRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "domain:{} type:{} data:{} ",
            self.domain, &self.record_type, &self.data
        )
    }
}

impl fmt::Display for ResourceRecordData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceRecordData::A(value) => write!(
                f,
                "ARecord: {}",
                Ipv4Addr::new(
                    (*value >> 24) as u8,
                    (*value >> 16) as u8,
                    (*value >> 8) as u8,
                    *value as u8
                )
            ),
            ResourceRecordData::AAAA(value) => write!(
                f,
                "AAAARecord: {}",
                Ipv6Addr::new(
                    (value >> 112) as u16,
                    (value >> 96) as u16,
                    (value >> 80) as u16,
                    (value >> 64) as u16,
                    (value >> 48) as u16,
                    (value >> 32) as u16,
                    (value >> 16) as u16,
                    (value & 0x00FF) as u16,
                )
            ),
            ResourceRecordData::CName(value) => write!(f, "CName: {}", value),
            ResourceRecordData::SOA(value) => write!(f, "SOARecord: {:?}", value),
            ResourceRecordData::MX(preference, exchange) => write!(
                f,
                "MXRecord: preference {:?}, exchange {:?}",
                preference, exchange
            ),
            ResourceRecordData::TXT(value) => write!(f, "TXTRecord: {:?}", value),
            ResourceRecordData::NS(value) => write!(f, "NSRecord: {:?}", value),
        }
    }
}
