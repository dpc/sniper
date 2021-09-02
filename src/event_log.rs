use crate::persistence::{self, Connection};
use anyhow::Result;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

use crate::service::{auction_house, bidding_engine, ui};

pub type EventId = String;
pub type EventIdRef<'a> = &'a str;

pub fn format_numeric_id(num: u64) -> String {
    format!("{:0>10}", num)
}

pub fn increment_id(id: &EventId) -> EventId {
    format_numeric_id(
        id.parse::<u64>()
            .expect("valid id")
            .checked_add(1)
            .expect("no overflow"),
    )
}

// TODO: This type makes everything cyclical:
// All services depend on it, and it depends
// on events of each of the services. Not a
// big deal for this small program, but something
// to take care of in a more realistic implementation.
#[derive(Clone, Debug)]
pub enum EventDetails {
    AuctionHouse(auction_house::Event),
    BiddingEngine(bidding_engine::Event),
    Ui(ui::Event),
}

pub struct Event {
    pub id: EventId,
    pub details: EventDetails,
}

pub trait Reader {
    type Persistence: persistence::Persistence;

    fn read_tr<'a>(
        &self,
        conn: &mut <<<Self as Reader>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        last: Option<EventId>,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<Event>>;
}

pub trait Writer {
    type Persistence: persistence::Persistence;

    fn write(
        &self,
        conn: &mut <<Self as Writer>::Persistence as persistence::Persistence>::Connection,
        events: &[EventDetails],
    ) -> Result<Option<EventId>>;

    fn write_tr<'a>(
        &self,
        conn: &mut <<<Self as Writer>::Persistence as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        events: &[EventDetails],
    ) -> Result<Option<EventId>>;
}

pub type SharedReader<P> = Arc<dyn Reader<Persistence = P> + Sync + Send + 'static>;
pub type SharedWriter<P> = Arc<dyn Writer<Persistence = P> + Sync + Send + 'static>;

type InMemoryLogInner = BTreeMap<String, EventDetails>;
pub struct InMemoryLog(RwLock<std::collections::BTreeMap<EventId, EventDetails>>);

impl InMemoryLog {
    pub fn get_last_id(inner: &InMemoryLogInner) -> Option<EventId> {
        inner.last_key_value().map(|(k, v)| k.to_owned())
    }

    pub fn get_next_id(inner: &InMemoryLogInner) -> EventId {
        increment_id(&Self::get_last_id(inner).unwrap_or_else(|| format_numeric_id(0)))
    }

    pub fn read<'a>(&'a self) -> RwLockReadGuard<'a, InMemoryLogInner> {
        self.0.read().expect("lock")
    }

    pub fn write<'a>(&'a self) -> RwLockWriteGuard<'a, InMemoryLogInner> {
        self.0.write().expect("lock")
    }

    fn write_events(&self, events: &[EventDetails]) -> Result<Option<EventId>> {
        let mut write = self.write();

        Ok(events
            .iter()
            .map(|e| {
                let next_id = Self::get_next_id(&write);
                write.insert(next_id.clone(), e.to_owned());
                next_id
            })
            .last()
            .or_else(|| Self::get_last_id(&write)))
    }
}

impl Reader for InMemoryLog {
    type Persistence = persistence::InMemoryPersistence;
    fn read_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        last: Option<EventId>,
        limit: usize,
        _timeout: Option<Duration>,
    ) -> Result<Vec<Event>> {
        let read = self.read();
        Ok(if let Some(last) = last {
            read.range(last..)
        } else {
            read.range::<EventId, _>(..)
        }
        .take(limit)
        .map(|(id, details)| Event {
            id: id.to_owned(),
            details: details.to_owned(),
        })
        .collect())
    }
}

impl Writer for InMemoryLog {
    type Persistence = persistence::InMemoryPersistence;

    fn write(
        &self,
        conn: &mut persistence::InMemoryConnection,
        events: &[EventDetails],
    ) -> Result<Option<EventId>> {
        let tr = conn.start_transaction()?;
        self.write_events(events)
    }

    fn write_tr<'a>(
        &self,
        _conn: &mut persistence::InMemoryTransaction,
        events: &[EventDetails],
    ) -> Result<Option<EventId>> {
        self.write_events(events)
    }
}

pub fn new_in_memory_shared() -> (
    SharedWriter<persistence::InMemoryPersistence>,
    SharedReader<persistence::InMemoryPersistence>,
) {
    let log = Arc::new(InMemoryLog(RwLock::new(BTreeMap::new())));
    (log.clone(), log)
}
