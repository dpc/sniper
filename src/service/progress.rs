use super::{ServiceId, ServiceIdRef};
use crate::event_log::{EventId, EventIdRef};

use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// A persistent store to keep track of the last processed event
pub trait ProgressTracker {
    fn store(&self, id: ServiceIdRef, event_id: EventIdRef) -> Result<()>;
    fn load(&self, id: ServiceIdRef) -> Result<Option<EventId>>;
}

pub type SharedProgressTracker = Arc<dyn ProgressTracker + Send + Sync + 'static>;

pub struct InMemoryProgressTracker {
    store: Mutex<BTreeMap<ServiceId, EventId>>,
}

impl InMemoryProgressTracker {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(BTreeMap::default()),
        }
    }

    pub fn new_shared() -> SharedProgressTracker {
        Arc::new(Self::new())
    }
}

impl ProgressTracker for InMemoryProgressTracker {
    fn store(&self, id: ServiceIdRef, event_id: EventIdRef) -> Result<()> {
        self.store
            .lock()
            .expect("lock")
            .insert(id.to_owned(), event_id.to_owned());
        Ok(())
    }
    fn load(&self, id: ServiceIdRef) -> Result<Option<EventId>> {
        Ok(self.store.lock().expect("lock").get(id).cloned())
    }
}
