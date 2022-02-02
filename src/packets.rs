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

#[derive(Debug, Clone)]
pub struct Frame {
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
    pub name_servers: Vec<Question>,
    pub additional_records: Vec<ResourceRecord>,
}

impl Frame {
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

            questions: vec![],
            answers: vec![],
            name_servers: vec![],
            additional_records: vec![],
        }
    }

    pub fn build_query(&self, id: u16) -> Frame {
        self.response_frame(PacketType::Query, id)
    }

    pub fn build_response(&self) -> Frame {
        self.response_frame(PacketType::Response, self.id)
    }

    pub fn add_question(&mut self, question: &Question) {
        self.questions.push(question.clone());
    }

    pub fn add_answer(&mut self, answer: &ResourceRecord) {
        self.answers.push(answer.clone());
    }
}
