use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uuid::Uuid;

#[derive(Clone)]
pub struct CachedRecord<T> {
    pub data: T,
    time_to_live: Duration,
    last_accessed: Instant,
}
pub struct Cache<T> {
    records: Arc<Mutex<HashMap<Uuid, CachedRecord<T>>>>,
}

impl<T> Cache<T>
where
    T: Clone + Send + 'static,
{
    pub fn new() -> Self {
        Cache {
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_record(&mut self, key: Uuid) -> Option<CachedRecord<T>> {
        let mut records = self.records.lock().unwrap();
        if let Some(record) = records.get(&key) {
            let mut mutable_record = record.clone();
            mutable_record.last_accessed = Instant::now();

            if Instant::now().duration_since(record.last_accessed) > record.time_to_live {
                records.remove(&key);
                return None;
            }

            return Some(record.clone());
        } else {
            None
        }
    }

    pub fn update_record(&self, key: Uuid, new_data: T) {
        let mut records = self.records.lock().unwrap();
        let record = CachedRecord {
            data: new_data,
            time_to_live: Duration::from_secs(60),
            last_accessed: Instant::now(),
        };

        records.insert(key, record);
    }

    pub fn cleanup_expired(&self) {
        let mut records = self.records.lock().unwrap();
        let now = Instant::now();
        records.retain(|_, record| now.duration_since(record.last_accessed) < record.time_to_live);
    }
}
