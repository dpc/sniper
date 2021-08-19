use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

pub enum Event {}

pub type EventId = String;

pub trait Reader {
    fn read(
        &self,
        last: Option<EventId>,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<Event>>;
}

pub trait Writer {
    fn write(&self, events: &[Event]) -> Result<()>;
}

pub type SharedReader = Arc<dyn Reader + Sync + Send + 'static>;
pub type SharedWriter = Arc<dyn Writer + Sync + Send + 'static>;
