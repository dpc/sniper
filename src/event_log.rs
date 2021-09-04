use crate::persistence::{self, Connection};
use anyhow::{format_err, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{convert::TryFrom, sync::Arc, time::Duration};

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
    type Persistence: persistence::Persistence;

    fn get_start_offset(&self) -> Result<Offset>;

    fn read_tr<'a>(
        &self,
        conn: &mut <<<Self as Reader>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<Event>)>;

    fn read<'a>(
        &self,
        conn: &mut <<Self as Reader>::Persistence as persistence::Persistence>::Connection,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<Event>)> {
        self.read_tr(&mut conn.start_transaction()?, offset, limit, timeout)
    }

    fn read_one_tr<'a>(
        &self,
        conn: &mut <<<Self as Reader>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        offset: Offset,
    ) -> Result<(Offset, Option<Event>)> {
        let (offset, v) = self.read_tr(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
    }

    fn read_one<'a>(
        &self,
        conn: &mut <<Self as Reader>::Persistence as persistence::Persistence>::Connection,
        offset: Offset,
    ) -> Result<(Offset, Option<Event>)> {
        let (offset, v) = self.read(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
    }
}

pub trait Writer {
    type Persistence: persistence::Persistence;

    fn write(
        &self,
        conn: &mut <<Self as Writer>::Persistence as persistence::Persistence>::Connection,
        events: &[EventDetails],
    ) -> Result<Offset> {
        self.write_tr(&mut conn.start_transaction()?, events)
    }

    fn write_tr<'a>(
        &self,
        conn: &mut <<<Self as Writer>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        events: &[EventDetails],
    ) -> Result<Offset>;
}

pub type SharedReader<P> = Arc<dyn Reader<Persistence = P> + Sync + Send + 'static>;
pub type SharedWriter<P> = Arc<dyn Writer<Persistence = P> + Sync + Send + 'static>;

type InMemoryLogInner = Vec<EventDetails>;

pub struct InMemoryLog {
    inner: RwLock<InMemoryLogInner>,
    condvar: util::CondvarAny,
}

impl InMemoryLog {
    pub fn read<'a>(&'a self) -> RwLockReadGuard<'a, InMemoryLogInner> {
        self.inner.read()
    }

    pub fn write<'a>(&'a self) -> RwLockWriteGuard<'a, InMemoryLogInner> {
        self.inner.write()
    }

    fn write_events(&self, events: &[EventDetails]) -> Result<Offset> {
        let mut write = self.write();

        write.extend_from_slice(events);
        self.condvar.notify_all();

        Ok(u64::try_from(write.len())?)
    }
}

impl Reader for InMemoryLog {
    type Persistence = persistence::InMemoryPersistence;
    fn read_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<Event>)> {
        let offset_usize = usize::try_from(offset)?;

        let mut read = self.read();

        if read.len() == offset_usize {
            if let Some(timeout) = timeout {
                self.condvar.wait_for(&mut read, timeout);
            } else {
                self.condvar.wait(&mut read);
            }
        }

        let res: Vec<_> = read
            .get(offset_usize..)
            .ok_or_else(|| format_err!("out of bounds"))?
            .iter()
            .take(limit)
            .enumerate()
            .map(|(i, e)| Event {
                offset: offset + u64::try_from(i).expect("no fail"),
                details: e.clone(),
            })
            .collect();

        Ok((offset + u64::try_from(res.len()).expect("no fail"), res))
    }

    fn get_start_offset(&self) -> Result<Offset> {
        Ok(0)
    }
}

impl Writer for InMemoryLog {
    type Persistence = persistence::InMemoryPersistence;

    fn write_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        events: &[EventDetails],
    ) -> Result<Offset> {
        self.write_events(events)
    }
}

pub fn new_in_memory_shared() -> (
    SharedWriter<persistence::InMemoryPersistence>,
    SharedReader<persistence::InMemoryPersistence>,
) {
    let log = Arc::new(InMemoryLog {
        inner: RwLock::new(Vec::new()),
        condvar: util::CondvarAny::default(),
    });
    (log.clone(), log)
}
