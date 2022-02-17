use std::{collections::HashMap, vec};

use chrono::Utc;
use tokio::sync::RwLock;

use crate::messages::packets::{
    Question, ResourceRecord, ResourceRecordClass, ResourceRecordData, ResourceRecordType,
};

#[derive(Debug)]
pub struct HashCache {
    map: RwLock<HashMap<CacheKey, Vec<CacheValue>>>,
}

#[derive(Debug)]
struct CacheValue {
    data: ResourceRecordData,
    time_to_live: u32,
    expiration: i64,
}

type CacheKey = (String, ResourceRecordType);

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

    pub async fn get(&self, record_type: ResourceRecordType, domain: &str) -> Vec<ResourceRecord> {
        // Get a read lock
        let map = self.map.read().await;

        // Find the value in the cache return none if it doesn't exist
        let results = map.get(&(domain.to_string(), record_type));

        match results {
            Some(results) => {
                // Filter out all the expired values
                return results
                    .iter()
                    .filter_map(|value| {
                        if value.is_expired() {
                            return None;
                        }

                        Some(value.to_resource_record(domain))
                    })
                    .collect();
            }

            // Just return an empty vector
            None => return vec![],
        }
    }

    pub async fn put_resource_records(
        &self,
        domain: &str,
        record_type: &ResourceRecordType,
        resource_records: &[ResourceRecord],
    ) {
        let mut map = self.map.write().await;

        let cache_key: CacheKey = (domain.to_string(), record_type.clone());

        let cache_value = resource_records
            .iter()
            .map(|record| {
                // Get the expiration time
                let expiration = Utc::now().timestamp() + record.time_to_live as i64;

                CacheValue {
                    data: record.data.clone(),
                    time_to_live: record.time_to_live,
                    expiration,
                }
            })
            .collect();

        map.insert(cache_key, cache_value);
    }

    pub async fn get_intersection(
        &self,
        questions: &[Question],
    ) -> (Vec<ResourceRecord>, Vec<Question>) {
        // Return vectors
        let mut excluded_questions = vec![];
        let mut answers = vec![];

        for question in questions.iter() {
            let mut found_answer = false;
            let found_records = self.get(question.question_type.clone(), &question.domain)
                .await;
                
            found_records.iter()
                .for_each(|value| {
                    answers.push(value.clone());
                    found_answer = true
                });

            if !found_answer {
                excluded_questions.push(question.clone())
            }
        }

        (answers, excluded_questions)
    }
}
