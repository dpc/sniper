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

impl Connection for InMemoryConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<OwnedTransaction<'a>> {
        Ok(Box::new(InMemoryTransaction {
            lock_guard: futures::executor::block_on(self.lock.lock()),
        }))
    }

    fn cast<'b>(&'b mut self) -> Caster<'b> {
        Caster::new(self)
    }
}

#[derive(Debug)]
pub struct InMemoryTransaction<'a> {
    #[allow(unused)] // used only by Drop
    lock_guard: MutexGuard<'a, ()>,
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

    fn cast<'b>(&'b mut self) -> Caster<'b>
    where
        'a: 'b,
    {
        Caster::new(unsafe {
            std::mem::transmute::<
                &'b mut InMemoryTransaction<'a>,
                &'b mut InMemoryTransaction<'static>,
            >(self) as &mut dyn Any
        })
    }
}
