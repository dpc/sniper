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
use thiserror::Error;

use anyhow::{bail, Result};
use std::{any::Any, sync::Arc};

pub trait Persistence: Send + Sync {
    fn get_connection(&self) -> Result<Box<dyn Connection>>;
}

pub type SharedPersistence = Arc<dyn Persistence>;
pub trait Connection: Any {
    fn start_transaction<'a>(&'a mut self) -> Result<OwnedTransaction<'a>>;

    fn cast<'b>(&'b mut self) -> Caster<'b>;
}

pub trait Transaction<'a> {
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;

    fn cast<'b>(&'b mut self) -> Caster<'b>
    where
        'a: 'b;
}

pub type OwnedTransaction<'a> = Box<dyn Transaction<'a> + 'a>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("wrong type")]
    WrongType,
}

pub struct Caster<'a>(&'a mut dyn Any);

impl<'a> Caster<'a> {
    pub fn new(any: &'a mut dyn Any) -> Self {
        Self(any)
    }

    // Returns `Result` so it's easier to handle with ? than an option
    pub fn as_mut<T: 'static>(self) -> Result<&'a mut T, Error> {
        self.0.downcast_mut::<T>().ok_or_else(|| Error::WrongType)
    }
}
