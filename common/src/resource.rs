use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct ResourcePool<T> {
    map: DashMap<String, T>,
    prefix: String,
    counter: AtomicU64,
}

impl<T> ResourcePool<T> {
    pub fn new(prefix: &str) -> Self {
        Self {
            map: DashMap::new(),
            prefix: prefix.to_string(),
            counter: AtomicU64::new(1),
        }
    }

    pub fn insert(&self, resource: T) -> String {
        let id = format!("{}_{}", self.prefix, self.counter.fetch_add(1, Ordering::Relaxed));
        self.map.insert(id.clone(), resource);
        id
    }

    pub fn get(&self, id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, T>> {
        self.map.get(id)
    }

    pub fn remove(&self, id: &str) -> Option<T> {
        self.map.remove(id).map(|(_, v)| v)
    }

    pub fn list(&self) -> Vec<String> {
        self.map.iter().map(|e| e.key().clone()).collect()
    }

    pub fn clear(&self) {
        self.map.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn with<F, R>(&self, id: &str, f: F) -> Option<R>
    where F: FnOnce(&T) -> R {
        self.map.get(id).map(|r| f(r.value()))
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.map.iter().map(|e| e.key().clone()).collect()
    }
}
