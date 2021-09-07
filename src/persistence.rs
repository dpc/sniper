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
pub mod in_memory;
pub mod postgres;

pub use self::{in_memory::*, postgres::*};

use anyhow::{bail, Result};
use std::{
    any::Any,
    sync::{Arc, RwLock, RwLockWriteGuard},
};

/// An instance of a persistence (store) that can hold data
///
/// Must be cloneable and thread-safe.
pub trait Persistence: Send + Sync + Clone {
    #[rustfmt::skip]
    type Connection: for<'a> Connection<Transaction<'a> = Self::Transaction<'a>>;
    type Transaction<'a>: Transaction<'a>;

    /// Get a connection to a store
    fn get_connection(&self) -> Result<Self::Connection>;
}

/// A connection to a database/persistence
pub trait Connection {
    type Transaction<'a>: Transaction<'a>
    where
        Self: 'a;
    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>>;
}

/// A database transaction to a database/persistence
pub trait Transaction<'a> {
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

pub trait ErasedPersistence {
    fn get_connection(&self) -> Result<Box<dyn ErasedConnection>>;
}

pub trait ErasedConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Box<dyn ErasedTransaction<'a> + 'a>>;
}

pub trait ErasedTransaction<'a>: Any + 'a {
    fn as_any(&mut self) -> &mut dyn Any;
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}
