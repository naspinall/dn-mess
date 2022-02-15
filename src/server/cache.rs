use std::collections::HashMap;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::packets::{
    Question, ResourceRecord, ResourceRecordClass, ResourceRecordData, ResourceRecordType,
};

pub struct HashCache {
    map: RwLock<HashMap<CacheKey, CacheValue>>,
}

#[derive(Eq, PartialEq, Hash)]
struct CacheKey {
    record_type: ResourceRecordType,
    domain: String,
}

struct CacheValue {
    data: ResourceRecordData,
    time_to_live: u32,
    expiration: i64,
}

impl CacheValue {
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expiration
    }

    pub fn to_resource_record(&self, domain: &str) -> ResourceRecord {
        ResourceRecord {
            domain: domain.to_string(),
            record_type: self.data.get_type(),
            class: ResourceRecordClass::InternetAddress,
            time_to_live: self.time_to_live,
            data: self.data.clone(),
        }
    }
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
    ) -> Option<ResourceRecord> {
        // Get a read lock
        let map = self.map.read().await;

        // Find the value in the cache return none if it doesn't exist
        let result = map.get(&CacheKey {
            record_type: record_type.clone(),
            domain: domain.to_string(),
        })?;

        // Check if the result has been expired
        if result.is_expired() {
            return None;
        }

        // Convert result to a resource record, return
        Some(result.to_resource_record(domain))
    }

    pub async fn put_resource_records(&self, domain: &str, record_type : &ResourceRecordType, resource_records: &[ResourceRecord]) {
        let mut map = self.map.write().await;
        resource_records.iter().for_each(|record| {
            let cache_key = CacheKey {
                domain: domain.to_string(),
                record_type: record_type.clone(),
            };

            // Get the expiration time
            let expiration = Utc::now().timestamp() + record.time_to_live as i64;

            let cache_value = CacheValue {
                data: record.data.clone(),
                time_to_live: record.time_to_live,
                expiration,
            };

            map.insert(cache_key, cache_value);
        })
    }

    pub async fn get_intersection(
        &self,
        questions: &[Question],
    ) -> (Vec<ResourceRecord>, Vec<Question>) {
        // Return vectors
        let mut excluded_questions = vec![];
        let mut answers = vec![];

        for question in questions.iter() {
            match self.get(&question.question_type, &question.domain).await {
                // Found so we'll add to the answers vector
                Some(data) => answers.push(data),

                // Not found so add to vector
                None => excluded_questions.push(question.clone()),
            }
        }

        (answers, excluded_questions)
    }
}
