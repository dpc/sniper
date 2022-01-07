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

use dyno::{Tag, Tagged};

use anyhow::{bail, Result};
use std::{any::Any, sync::Arc};

/// An interface of any persistence
///
/// Persistence is anything that a Repository implementation could
/// use to store data.
pub trait Persistence: Send + Sync {
    /// Get a connection to persistence
    fn get_connection(&self) -> Result<OwnedConnection>;
}

pub type SharedPersistence = Arc<dyn Persistence>;

pub trait Connection: Any {
    fn start_transaction<'a>(&'a mut self) -> Result<OwnedTransaction<'a>>;

    fn cast<'borrow>(&'borrow mut self) -> Caster<'borrow, 'static>;
}

pub type OwnedConnection = Box<dyn Connection>;

pub trait Transaction<'a> {
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;

    fn cast<'b>(&'b mut self) -> Caster<'b, 'a>
    where
        'a: 'b;
}

pub type OwnedTransaction<'a> = Box<dyn Transaction<'a> + 'a>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("wrong type")]
    WrongType,
}

/// Dynamic cast helper
///
/// This struct allows an implementation of a Repository
/// to cast at runtime a type-erased [`Transaction`] or [`Connection`]
/// instance to back to a concrete type that it needs and expects.
///
/// # Safety
/// See https://users.rust-lang.org/t/help-with-using-any-to-cast-t-a-back-and-forth/69900/8
///
/// The safety is enforced by the fact that `Caster` pinky-promises to never
/// allow any reference other that `&'caster mut T` out of itself, and
/// `'a` must always outlive `'caster` or borrowck will be upset.
pub struct Caster<'borrow, 'value>(&'borrow mut (dyn Tagged<'value> + 'value));

impl<'borrow, 'value> Caster<'borrow, 'value> {
    pub fn new<I: Tag<'value>>(any: &'borrow mut I::Type) -> Self {
        Self(<dyn Tagged>::tag_mut::<I>(any))
    }

    // Returns `Result` so it's easier to handle with ? than an option
    pub fn as_mut<I: Tag<'value>>(self) -> Result<&'borrow mut I::Type, Error> {
        self.0.downcast_mut::<I>().ok_or_else(|| Error::WrongType)
    }
}
