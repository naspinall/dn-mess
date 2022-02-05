use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::packets::{
    Question, ResourceRecord, ResourceRecordClass, ResourceRecordData, ResourceRecordType,
};

pub struct HashCache {
    map: RwLock<HashMap<(ResourceRecordType, String), (ResourceRecordData, u32)>>,
}

impl HashCache {
    pub fn new() -> HashCache {
        HashCache {
            map: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(
        &self,
        record_type: &ResourceRecordType,
        domain: &str,
    ) -> Option<(ResourceRecordData, u32)> {
        let map = self.map.read().await;
        let data = map.get(&(record_type.clone(), domain.to_string()))?;

        Some(data.clone())
    }

    pub async fn put_resource_records(&self, resource_records: &Vec<ResourceRecord>) {
        let mut map = self.map.write().await;
        resource_records.iter().for_each(|record| {
            map.insert(
                (record.record_type.clone(), record.domain.to_string()),
                (record.data.clone(), record.time_to_live),
            );
        })
    }

    pub async fn get_intersection(
        &self,
        questions: &Vec<Question>,
    ) -> (Vec<ResourceRecord>, Vec<Question>) {
        // Return vectors
        let mut excluded_questions = vec![];
        let mut answers = vec![];

        for question in questions.iter() {
            match self.get(&question.question_type, &question.domain).await {
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
