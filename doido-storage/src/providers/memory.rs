//! In-memory [`Service`] — stores object bytes in a `HashMap`, the storage
//! analogue of `doido_cache::MemoryStore`.
//!
//! It keeps nothing on disk and is wiped when the process exits, so it's ideal
//! for tests and quick local development. Like disk, it has no native access URL
//! and is served through the app's signed [`crate::serving`] routes.

use crate::error::StorageError;
use crate::service::Service;
use doido_core::Result;
use std::collections::HashMap;
use std::sync::Mutex;

/// A [`Service`] that keeps every object in memory.
pub struct MemoryService {
    name: String,
    data: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemoryService {
    /// Create an empty in-memory service named `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryService {
    fn default() -> Self {
        Self::new("memory")
    }
}

#[async_trait::async_trait]
impl Service for MemoryService {
    fn name(&self) -> &str {
        &self.name
    }

    async fn upload(&self, key: &str, data: Vec<u8>, _content_type: Option<&str>) -> Result<()> {
        self.data.lock().unwrap().insert(key.to_string(), data);
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        self.data
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(key.to_string()).into())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.data.lock().unwrap().contains_key(key))
    }

    async fn size(&self, key: &str) -> Result<u64> {
        self.data
            .lock()
            .unwrap()
            .get(key)
            .map(|b| b.len() as u64)
            .ok_or_else(|| StorageError::NotFound(key.to_string()).into())
    }
}
