use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crate::persistence;

use crate::service::{auction_house, bidding_engine, ui};

pub type EventId = String;
pub type EventIdRef<'a> = &'a str;

// TODO: This type makes everything cyclical:
// All services depend on it, and it depends
// on events of each of the services. Not a
// big deal for this small program, but something
// to take care of in a more realistic implementation.
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
    type Persistence : persistence::Persistence;

    fn read(
        &self,
        last: Option<EventId>,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<Event>>;
}

pub trait Writer {
    type Persistence : persistence::Persistence;

    fn write(&self, events: &[EventDetails]) -> Result<()>;
}

pub type SharedReader<P> = Arc<dyn Reader<Persistence=P> + Sync + Send + 'static>;
pub type SharedWriter<P> = Arc<dyn Writer<Persistence=P> + Sync + Send + 'static>;

// TODO: address double `Arc`?
pub struct InMemoryLogReader(Arc<RwLock<std::collections::BTreeMap<EventId, EventDetails>>>);

impl Reader for InMemoryLogReader {
    type Persistence = persistence::InMemoryPersistence;
    fn read(
        &self,
        last: Option<EventId>,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<Event>> {
        todo!()
    }
}

pub struct InMemoryLogWriter(Arc<RwLock<std::collections::BTreeMap<EventId, EventDetails>>>);

impl Writer for InMemoryLogWriter {
    type Persistence = persistence::InMemoryPersistence;
    fn write(&self, events: &[EventDetails]) -> Result<()> {
        todo!()
    }
}

pub fn new_in_memory_shared() -> (SharedWriter<persistence::InMemoryPersistence>, SharedReader<persistence::InMemoryPersistence>) {
    let log = Arc::new(RwLock::new(BTreeMap::new()));
    (
        Arc::new(InMemoryLogWriter(log.clone())),
        Arc::new(InMemoryLogReader(log)),
    )
}
