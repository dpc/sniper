//! Database persistence traitsi
//!
//! OK, so this one is complex. Expressing atomic transactions
//! spaning accross multiple stores/repositories in hexagonal
//! architecture is not a simple thing in any language.
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

/// Trait unifying `Connection` and `Transaction` under one umbrealla
pub trait GenericConnection {}

/// A connection to a database/persistence
pub trait Connection: GenericConnection {
    type Transaction<'a>: Transaction
    where
        Self: 'a;
    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>>;
}

/// A database transaction to a database/persistence
pub trait Transaction: GenericConnection {
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

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

impl GenericConnection for InMemoryConnection {}

impl Connection for InMemoryConnection {
    type Transaction<'a> = InMemoryTransaction;

    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(InMemoryTransaction)
    }
}

#[derive(Default, Debug)]
pub struct InMemoryTransaction;

impl GenericConnection for InMemoryTransaction {}

impl Transaction for InMemoryTransaction {
    fn commit(self) -> Result<()> {
        Ok(())
    }

    fn rollback(self) -> Result<()> {
        bail!("Not supported")
    }
}
