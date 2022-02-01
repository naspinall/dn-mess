use std::collections::HashMap;

use crate::packets::{ResourceRecordData, ResourceRecordType};

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
}
