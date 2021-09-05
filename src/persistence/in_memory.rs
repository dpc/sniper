use std::collections::BTreeMap;

use parking_lot::Mutex;

use crate::auction::ItemId;

use super::*;

pub fn new() -> Persistence {
    Persistence(Arc::new(InMemoryPersistence::default()))
}

/// Fake in-memory persistence.
///
/// Useful for unit-tests.
#[derive(Debug, Default)]
struct InMemoryPersistence(RwLock<BTreeMap<ItemId, AuctionBiddingState>>);

impl PersistenceImpl for InMemoryPersistence {
    fn get_connection(self: Arc<Self>) -> Result<Connection> {
        let res = Connection(Box::new(InMemoryConnection { lock: self.clone() }));
        Ok(res)
    }
}

#[derive(Default, Debug)]
struct InMemoryConnection {
    lock: Arc<InMemoryPersistence>,
}

impl ConnectionImpl for InMemoryConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Transaction<'a>> {
        let res = Transaction(Box::new(InMemoryTransaction {
            lock_guard: self.lock.0.write().expect("lock to work"),
        }));
        Ok(res)
    }
}

#[derive(Debug)]
struct InMemoryTransaction<'a> {
    lock_guard: RwLockWriteGuard<'a, BTreeMap<ItemId, AuctionBiddingState>>,
}

impl<'a> TransactionImpl<'a> for InMemoryTransaction<'a> {
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

    fn load_tr(&mut self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>> {
        Ok(self.lock_guard.get(item_id).cloned())
    }

    fn store_tr(&mut self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()> {
        self.lock_guard.insert(item_id.to_owned(), state);
        Ok(())
    }
}
