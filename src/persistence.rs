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

use anyhow::{bail, Result};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::{auction::ItemIdRef, service::bidding_engine::AuctionBiddingState};

#[derive(Clone)]
pub struct Persistence(Arc<dyn PersistenceImpl>);
pub struct Connection(Box<dyn ConnectionImpl>);
pub struct Transaction<'a>(Box<dyn TransactionImpl<'a> + 'a>);

impl Persistence {
    pub fn get_connection(&self) -> Result<Connection> {
        self.0.clone().get_connection()
    }
}
impl Connection {
    pub fn start_transaction(&mut self) -> Result<Transaction<'_>> {
        self.0.start_transaction()
    }

    pub fn load(&mut self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>> {
        let mut transaction = self.start_transaction()?;
        transaction.load_tr(item_id)
    }

    pub fn store(&mut self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()> {
        let mut transaction = self.start_transaction()?;
        transaction.store_tr(item_id, state)
    }
}
impl<'a> Transaction<'a> {
    pub fn commit(self) -> Result<()> {
        self.0.commit()
    }
    pub fn rollback(self) -> Result<()> {
        self.0.rollback()
    }
    pub fn load_tr(
        &mut self,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<AuctionBiddingState>> {
        self.0.load_tr(item_id)
    }

    pub fn store_tr(
        &mut self,
        item_id: crate::auction::ItemIdRef,
        state: AuctionBiddingState,
    ) -> anyhow::Result<()> {
        self.0.store_tr(item_id, state)
    }
}

pub trait PersistenceImpl: Send + Sync {
    fn get_connection(self: Arc<Self>) -> Result<Connection>;
}
pub trait ConnectionImpl {
    fn start_transaction<'a>(&'a mut self) -> Result<Transaction<'a>>;
}
pub trait TransactionImpl<'a> {
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;
    fn load_tr(
        &mut self,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<AuctionBiddingState>>;

    fn store_tr(
        &mut self,
        _item_id: crate::auction::ItemIdRef,
        _state: AuctionBiddingState,
    ) -> anyhow::Result<()>;
}

