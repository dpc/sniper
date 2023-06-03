use super::*;
use futures;
use tokio::sync::{Mutex, MutexGuard};

/// Fake in-memory persistence.
///
/// Useful for unit-tests.
#[derive(Debug, Clone)]
pub struct InMemoryPersistence {
    lock: Arc<Mutex<()>>,
}

impl InMemoryPersistence {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }
}

impl Persistence for InMemoryPersistence {
    fn get_connection(&self) -> Result<Box<dyn Connection>> {
        Ok(Box::new(InMemoryConnection {
            lock: self.lock.clone(),
        }))
    }
}

#[derive(Default, Debug)]
pub struct InMemoryConnection {
    lock: Arc<Mutex<()>>,
}

impl<'a> dyno::Tag<'a> for InMemoryConnection {
    type Type = InMemoryConnection;
}

impl Connection for InMemoryConnection {
    fn start_transaction(&mut self) -> Result<OwnedTransaction<'_>> {
        Ok(Box::new(InMemoryTransaction {
            _lock_guard: futures::executor::block_on(self.lock.lock()),
        }))
    }

    fn cast(&mut self) -> Caster<'_, 'static> {
        Caster::new::<InMemoryConnection>(self)
    }
}

#[derive(Debug)]
pub struct InMemoryTransaction<'a> {
    _lock_guard: MutexGuard<'a, ()>,
}

impl<'a> dyno::Tag<'a> for InMemoryTransaction<'static> {
    type Type = InMemoryTransaction<'a>;
}

impl<'a> Transaction<'a> for InMemoryTransaction<'a> {
    fn commit(self: Box<Self>) -> Result<()> {
        Ok(())
    }

    // TODO: simulating rollbacks in a general way is not trivial
    // and it would require all the `InMemory*` stores implementations
    // to register previous value when creating the transaction or
    // something like this.
    fn rollback(self: Box<Self>) -> Result<()> {
        bail!("Not supported")
    }

    fn cast<'caster>(&'caster mut self) -> Caster<'caster, 'a>
    where
        'a: 'caster,
    {
        Caster::new::<InMemoryTransaction<'static>>(self)
    }
}
