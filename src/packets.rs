#[derive(Debug)]
pub enum PacketType {
    Question,
    Answer,
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
pub enum AnswerData {
    ARecord(u32),
    CName(String),
}

#[derive(Debug)]
pub struct HeaderPacket {
    pub id: u16,
    pub packet_type: PacketType,
    pub op_code: u8,

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
pub struct AnswerPacket {
    pub domain: String,
    pub answer_type: QuestionType,
    pub class: QuestionClass,
    pub time_to_live: u32,
    pub answer_data: AnswerData,
}
