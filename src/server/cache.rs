use std::collections::HashMap;

use crate::packets::{ResourceRecordData, ResourceRecordType};

pub struct HashCache {
    map : HashMap<(ResourceRecordType, String), ResourceRecordData>
}

impl HashCache {
    pub fn new() -> HashCache {
        HashCache{
            map: HashMap::new(),
        }
    }

    pub fn get(&self, record_type: &ResourceRecordType, domain: &str) -> Option<ResourceRecordData> {
        let data = self.map.get(&(record_type.clone(), domain.to_string()))?;
        
        Some(
            data.clone()
        )
    }

    pub fn put(&mut self, record_type: &ResourceRecordType, domain: &str, data: &ResourceRecordData) {
        self.map.insert((record_type.clone(), domain.to_string()), data.clone());
    }
}