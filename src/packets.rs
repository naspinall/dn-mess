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

#[derive(Debug, Clone)]
pub enum ResourceRecordData {
    ARecord(u32),
    AAAARecord(u128),
    CName(String),
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
pub struct QuestionPacket {
    pub domain: String,
    pub question_type: ResourceRecordType,
    pub class: QuestionClass,
}

#[derive(Debug, Clone)]
pub struct ResourceRecordPacket {
    pub domain: String,
    pub record_type: ResourceRecordType,
    pub class: ResourceRecordClass,
    pub time_to_live: u32,
    pub record_data: ResourceRecordData,
}

#[derive(Debug)]
pub struct Frame {
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

    pub questions: Vec<QuestionPacket>,
    pub answers: Vec<ResourceRecordPacket>,
}

impl Frame {
    pub fn new(id: u16, packet_type: PacketType) -> Frame {
        Frame {
            id,
            packet_type,
            // Only support standard queries
            op_code: 0,
            // These options will be set elsewhere
            authoritative_answer: false,
            truncation: false,
            recursion_desired: true,
            recursion_available: false,
            // Default to no error
            response_code: ResponseCode::None,

            // Zero out
            question_count: 0,
            answer_count: 0,
            name_server_count: 0,
            additional_records_count: 0,

            questions: vec![],
            answers: vec![],
        }
    }

    fn response_frame(&self, response_type: PacketType, id: u16) -> Frame {
        Frame {
            id,
            packet_type: response_type,
            // Only support standard queries
            op_code: 0,
            // These options will be set elsewhere
            authoritative_answer: false,
            truncation: false,
            recursion_desired: false,
            recursion_available: false,
            // Default to no error
            response_code: ResponseCode::None,

            // Zero out
            question_count: 0,
            answer_count: 0,
            name_server_count: 0,
            additional_records_count: 0,

            questions: vec![],
            answers: vec![],
        }
    }

    pub fn build_query(&self, id: u16) -> Frame {
        self.response_frame(PacketType::Query, id)
    }

    pub fn build_response(&self) -> Frame {
        self.response_frame(PacketType::Response, self.id)
    }

    pub fn add_question(&mut self, question: &QuestionPacket) {
        self.questions.push(question.clone());
        self.question_count += 1;
    }

    pub fn add_answer(&mut self, answer: &ResourceRecordPacket) {
        self.answers.push(answer.clone());
        self.answer_count += 1;
    }
}
