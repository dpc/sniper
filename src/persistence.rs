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

    fn cast<'b>(&'b mut self) -> Caster<'b>;
}

pub type OwnedConnection = Box<dyn Connection>;

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

/// Dynamic cast helper
///
/// This struct allows an implementation of a Repository
/// to cast at runtime a type-erased [`Transaction`] or [`Connection`]
/// instance to back to a concrete type that it needs and expects.
pub struct Caster<'caster>(&'caster mut dyn Any);

impl<'caster> Caster<'caster> {
    pub fn new(any: &'caster mut dyn Any) -> Self {
        Self(any)
    }

    /// Create a caster by transmutting `T<'a>` into a `T<'static>`
    ///
    /// The whole `transmute` sheningans are only required because `Any` requires
    /// `'static`, while we have some `T<'a>` that we need to store as temporarily
    /// cast to `Any` and cast back to `T<'b>` where `'a: 'b`. `
    ///
    /// The reason `Any` requires `'static` is that downcasting things in the presence
    /// of non-static lifetimes is non-trivial, and no one ever researched and/or implemented
    /// it.
    ///
    /// # Safety
    ///
    /// The safety is enforced by the fact that `Caster` pinky-promises to never
    /// allow any reference other that `&'caster mut T` out of itself, and
    /// `'a` must always outlive `'caster` or borrowck will be upset.
    ///
    /// The reason that this function is unsafe is that `Ta` and `Tstatic` must be the
    /// same type, only with different lifetimes, and there seem to be no way to enforce
    /// it.
    pub unsafe fn new_transmute<'a, Ta: 'a, Tstatic: 'static>(t: &'caster mut Ta) -> Self
    where
        'a: 'caster,
    {
        Self(std::mem::transmute::<&'caster mut Ta, &'caster mut Tstatic>(t) as &mut dyn Any)
    }

    // Returns `Result` so it's easier to handle with ? than an option
    pub fn as_mut<T: 'static>(self) -> Result<&'caster mut T, Error> {
        self.0.downcast_mut::<T>().ok_or_else(|| Error::WrongType)
    }
}
