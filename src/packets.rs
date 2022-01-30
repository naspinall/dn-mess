#[derive(Debug)]
pub enum PacketType {
    Query,
    Response,
}

#[derive(Debug)]

pub enum QuestionType {
    ARecord,
    CNameRecord,
    MXRecord,
    NameServersRecord,
    Unimplemented,
}
#[derive(Debug)]

pub enum QuestionClass {
    InternetAddress,
    Unimplemented,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum ResourceRecordClass {
    InternetAddress,
    Unimplemented,
}

#[derive(Debug)]
pub enum ResourceRecordData {
    ARecord(u32),
    AAAARecord(u128),
    CName(String),
}

#[derive(Debug)]
pub enum ResponseCode {
    None,
    FormatError,
    ServerError,
    NameError,
    NotImplemented,
    Refused,
}
#[derive(Debug)]
pub struct HeaderPacket {
    pub id: u16,
    pub packet_type: PacketType,
    pub op_code: u8,
    pub authoritative_answer: bool,
    pub truncation: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub response_code: ResponseCode,

    pub question_count: u16,
    pub answer_count: u16,
    pub name_server_count: u16,
    pub additional_records_count: u16,
}

#[derive(Debug)]
pub struct QuestionPacket {
    pub domain: String,
    pub question_type: QuestionType,
    pub class: QuestionClass,
}

#[derive(Debug)]
pub struct ResourceRecordPacket {
    pub domain: String,
    pub record_type: ResourceRecordType,
    pub class: ResourceRecordClass,
    pub time_to_live: u32,
    pub record_data: ResourceRecordData,
}
