use super::{ServiceId, ServiceIdRef};
use crate::{event_log::Offset, persistence};
use anyhow::format_err;

use anyhow::Result;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

/// A persistent store to keep track of the last processed event
pub trait ProgressTracker {
    type Persistence: persistence::Persistence;
    fn load(
        &self,
        conn: &mut <<Self as ProgressTracker>::Persistence as persistence::Persistence>::Connection,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>>;

    fn store_tr<'a>(
        &self,
        conn: &mut <<<Self as ProgressTracker>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        id: ServiceIdRef,
        offset: Offset,
    ) -> Result<()>;
    fn load_tr<'a>(
        &self,
        conn: &mut <<<Self as ProgressTracker>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>>;
}

pub type SharedProgressTracker<P> =
    Arc<dyn ProgressTracker<Persistence = P> + Send + Sync + 'static>;

pub struct InMemoryProgressTracker {
    store: Mutex<BTreeMap<ServiceId, Offset>>,
}

impl InMemoryProgressTracker {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(BTreeMap::default()),
        }
    }

    pub fn new_shared() -> SharedProgressTracker<persistence::InMemoryPersistence> {
        Arc::new(Self::new())
    }

    pub fn lock(&self) -> Result<MutexGuard<'_, BTreeMap<ServiceId, Offset>>> {
        self.store
            .lock()
            .map_err(|_e| format_err!("mutex poisoned"))
    }
}

impl ProgressTracker for InMemoryProgressTracker {
    type Persistence = persistence::InMemoryPersistence;

    fn load<'a>(
        &self,
        _conn: &mut persistence::InMemoryConnection,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>> {
        Ok(self.lock()?.get(id).cloned())
    }

    fn store_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        id: ServiceIdRef,
        event_id: Offset,
    ) -> Result<()> {
        self.lock()?.insert(id.to_owned(), event_id.to_owned());
        Ok(())
    }

    fn load_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>> {
        Ok(self.lock()?.get(id).cloned())
    }
}
