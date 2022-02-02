use std::collections::HashMap;

use crate::packets::{
    Question, ResourceRecord, ResourceRecordClass, ResourceRecordData, ResourceRecordType,
};

pub struct HashCache {
    map: HashMap<(ResourceRecordType, String), (ResourceRecordData, u32)>,
}

impl HashCache {
    pub fn new() -> HashCache {
        HashCache {
            map: HashMap::new(),
        }
    }

    pub fn get(
        &self,
        record_type: &ResourceRecordType,
        domain: &str,
    ) -> Option<(ResourceRecordData, u32)> {
        let data = self.map.get(&(record_type.clone(), domain.to_string()))?;

        Some(data.clone())
    }

    pub fn put(
        &mut self,
        record_type: &ResourceRecordType,
        domain: &str,
        data: &ResourceRecordData,
        time_to_live: u32,
    ) {
        self.map.insert(
            (record_type.clone(), domain.to_string()),
            (data.clone(), time_to_live),
        );
    }

    pub fn put_resource_records(&mut self, resource_records: &Vec<ResourceRecord>) {
        resource_records.iter().for_each(|record| {
            self.put(
                &record.record_type,
                &record.domain,
                &record.data,
                record.time_to_live,
            )
        })
    }

    pub fn get_intersection(
        &self,
        questions: &Vec<Question>,
    ) -> (Vec<ResourceRecord>, Vec<Question>) {
        // Return vectors
        let mut excluded_questions = vec![];
        let mut answers = vec![];

        for question in questions.iter() {
            match self.get(&question.question_type, &question.domain) {
                // Found so we'll add to the answers vector
                Some((data, time_to_live)) => answers.push(ResourceRecord {
                    domain: question.domain.clone(),
                    data,
                    time_to_live,
                    record_type: question.question_type.clone(),
                    class: ResourceRecordClass::InternetAddress,
                }),
                // Not found so add to vector
                None => excluded_questions.push(question.clone()),
            }
        }

        (answers, excluded_questions)
    }
}
