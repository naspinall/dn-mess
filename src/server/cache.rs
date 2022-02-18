use std::{collections::HashMap, vec};

use chrono::Utc;
use tokio::sync::RwLock;

use crate::messages::packets::{
    Question, ResourceRecord, ResourceRecordClass, ResourceRecordData, ResourceRecordType,
};

type CacheKey = (String, ResourceRecordType);

#[derive(Debug)]
pub struct HashCache {
    map: RwLock<HashMap<CacheKey, Vec<CacheValue>>>,
}

#[derive(Debug, PartialEq)]
struct CacheValue {
    data: ResourceRecordData,
    time_to_live: u32,
    expiration: i64,
}

impl CacheValue {
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expiration
    }

    pub fn from_resource_record(record: &ResourceRecord) -> CacheValue {
        CacheValue {
            data: record.data.clone(),
            time_to_live: record.time_to_live,
            expiration: Utc::now().timestamp() + record.time_to_live as i64,
        }
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
        record_type: ResourceRecordType,
        domain: &str,
    ) -> Option<Vec<ResourceRecord>> {
        // Get a read lock
        let map = self.map.read().await;

        // Find the value in the cache return none if it doesn't exist
        let results = map.get(&(domain.to_string(), record_type));

        match results {
            Some(results) => {
                // Filter out all the expired values

                let return_results: Vec<ResourceRecord> = results
                    .iter()
                    .filter_map(|value| {
                        if value.is_expired() {
                            return None;
                        }

                        Some(value.to_resource_record(domain))
                    })
                    .collect();

                // If empty, just return None
                if return_results.is_empty() {
                    return None;
                }

                return Some(return_results);
            }

            // Just return an empty vector
            None => return None,
        }
    }

    pub async fn put_resource_records(&self, domain: &str, resource_records: &Vec<ResourceRecord>) {
        // Get write lock
        let mut map = self.map.write().await;

        // Add all records to the cache
        resource_records.iter().for_each(|record| {
            // Make key
            let cache_key: CacheKey = (domain.to_string(), record.record_type.clone());
            let cache_value = CacheValue::from_resource_record(record);

            // Check if already in cache
            if !map.contains_key(&cache_key) {
                // Insert the value, we are done
                map.insert(cache_key, vec![cache_value]);
                return;
            }

            // Add to the list of existing records if not already contained
            match map.get_mut(&cache_key) {
                Some(value) => {
                    // Already cached, ignore it
                    if value.contains(&cache_value) {
                        return;
                    }

                    // Otherwise add to the list of values
                    value.push(cache_value)
                }
                // Do nothing
                None => return,
            }
        })
    }
}
