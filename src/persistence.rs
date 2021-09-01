//! Database persistence traitsi
//!
//! OK, so this one is complex. Expressing atomic transactions
//! spaning accross multiple stores/repositories in a hexagonal
//! architecture is not a simple thing in any programming language.
//!
//! Some discussion:
//!
//! * https://www.reddit.com/r/rust/comments/p9amqt/hexagonal_architecture_in_rust_1/h9ypjoo?utm_source=share&utm_medium=web2x&context=3
//! * https://www.reddit.com/r/golang/comments/i1vy4s/ddd_vs_db_transactions_how_to_reconcile/
pub mod postgres;

use anyhow::Result;
use anyhow::bail;

/// An instance of a persistence (store) that can hold data
///
/// Must be cloneable and thread-safe.
pub trait Persistence : Send + Sync + Clone {
    type Connection: Connection;

    /// Get a connection to a store
    fn get_connection(&self) -> Result<Self::Connection>;
}

/// A connection to a database/persistence
pub trait Connection {
    type Transaction<'a>: Transaction
    where
        Self: 'a;
    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>>;
}

/// A database transaction to a database/persistence
pub trait Transaction {
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

/// Fake in-memory persistence.
///
/// Useful for unit-tests.
#[derive(Default, Debug, Clone)]
pub struct InMemoryPersistence {}

impl InMemoryPersistence {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Persistence for InMemoryPersistence {
    type Connection = InMemoryConnection;

    fn get_connection(&self) -> Result<Self::Connection> {
        Ok(InMemoryConnection::default())
    }
}

#[derive(Default, Debug)]
pub struct InMemoryConnection {}


impl Connection for InMemoryConnection {
    type Transaction<'a> = InMemoryTransaction;

    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(InMemoryTransaction)
    }
}

#[derive(Default, Debug)]
pub struct InMemoryTransaction;

impl Transaction for InMemoryTransaction {
    fn commit(self) -> Result<()> {
        Ok(())
    }

    // TODO: simulating rollbacks in a general way is not trivial
    // and it would require all the `InMemory*` stores implementations
    // to register previous value when creating the transaction or
    // something like this.
    fn rollback(self) -> Result<()> {
        bail!("Not supported")
    }
}
