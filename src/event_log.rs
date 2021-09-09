use crate::persistence::{Connection, Transaction};
use anyhow::{format_err, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{convert::TryFrom, sync::Arc, time::Duration};

mod in_memory;
pub use self::in_memory::*;

mod util {
    use parking_lot::{Condvar, Mutex, RwLockReadGuard};
    use std::time::Duration;

    #[derive(Default)]
    // https://github.com/Amanieu/parking_lot/issues/165#issuecomment-515991706
    pub struct CondvarAny {
        c: Condvar,
        m: Mutex<()>,
    }

    impl CondvarAny {
        pub fn wait<T>(&self, g: &mut RwLockReadGuard<'_, T>) {
            let guard = self.m.lock();
            RwLockReadGuard::unlocked(g, || {
                // Move the guard in so it gets unlocked before we re-lock g
                let mut guard = guard;
                self.c.wait(&mut guard);
            });
        }

        pub fn wait_for<T>(&self, g: &mut RwLockReadGuard<'_, T>, timeout: Duration) {
            let guard = self.m.lock();
            RwLockReadGuard::unlocked(g, || {
                // Move the guard in so it gets unlocked before we re-lock g
                let mut guard = guard;
                self.c.wait_for(&mut guard, timeout);
            });
        }

        pub fn notify_all(&self) -> usize {
            self.c.notify_all()
        }
    }
}

use crate::service::{auction_house, bidding_engine, ui};

pub type Offset = u64;

// TODO: This type makes everything cyclical:
// All services depend on it, and it depends
// on events of each of the services. Not a
// big deal for this small program, but something
// to take care of in a more realistic implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventDetails {
    AuctionHouse(auction_house::Event),
    BiddingEngine(bidding_engine::Event),
    Ui(ui::Event),
    #[cfg(test)]
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub offset: Offset,
    pub details: EventDetails,
}

pub trait Reader {
    fn get_start_offset(&self) -> Result<Offset>;

    fn read_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<Event>)>;

    fn read<'a>(
        &self,
        conn: &mut dyn Connection,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<Event>)> {
        self.read_tr(&mut *conn.start_transaction()?, offset, limit, timeout)
    }

    fn read_one_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        offset: Offset,
    ) -> Result<(Offset, Option<Event>)> {
        let (offset, v) = self.read_tr(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
    }

    fn read_one<'a>(
        &self,
        conn: &mut dyn Connection,
        offset: Offset,
    ) -> Result<(Offset, Option<Event>)> {
        let (offset, v) = self.read(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
    }
}

pub trait Writer {
    fn write(&self, conn: &mut dyn Connection, events: &[EventDetails]) -> Result<Offset> {
        self.write_tr(&mut *conn.start_transaction()?, events)
    }

    fn write_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        events: &[EventDetails],
    ) -> Result<Offset>;
}

pub type SharedReader = Arc<dyn Reader + Sync + Send + 'static>;
pub type SharedWriter = Arc<dyn Writer + Sync + Send + 'static>;
