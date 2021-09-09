use crate::persistence::{InMemoryConnection, InMemoryTransaction};
use anyhow::Result;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

use super::*;

pub struct InMemoryProgressTracker {
    store: Mutex<BTreeMap<ServiceId, Offset>>,
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

    pub fn lock(&self) -> Result<MutexGuard<'_, BTreeMap<ServiceId, Offset>>> {
        self.store
            .lock()
            .map_err(|_e| format_err!("mutex poisoned"))
    }
}

impl ProgressTracker for InMemoryProgressTracker {
    fn load<'a>(&self, conn: &mut dyn Connection, id: ServiceIdRef) -> Result<Option<Offset>> {
        conn.cast().as_mut::<InMemoryConnection>()?;
        Ok(self.lock()?.get(id).cloned())
    }

    fn store_tr<'a>(
        &self,
        conn: &mut dyn Transaction,
        id: ServiceIdRef,
        event_id: Offset,
    ) -> Result<()> {
        conn.cast().as_mut::<InMemoryTransaction>()?;

        self.lock()?.insert(id.to_owned(), event_id.to_owned());
        Ok(())
    }

    fn load_tr<'a>(&self, conn: &mut dyn Transaction, id: ServiceIdRef) -> Result<Option<Offset>> {
        conn.cast().as_mut::<InMemoryTransaction>()?;
        Ok(self.lock()?.get(id).cloned())
    }
}
