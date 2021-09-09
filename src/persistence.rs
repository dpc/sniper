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
use std::sync::{Arc, RwLock, RwLockWriteGuard};

pub trait Persistence: Send + Sync {
    fn get_connection(&self) -> Result<Box<dyn Connection>>;
}

pub type SharedPersistence = Arc<dyn Persistence>;
pub trait Connection {
    fn start_transaction<'a>(&'a mut self) -> Result<Box<dyn Transaction<'a> + 'a>>;
}

pub trait Transaction<'a> {
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}
