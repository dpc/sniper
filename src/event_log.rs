use crate::{
    event::Event,
    persistence::{Connection, Transaction},
};
use anyhow::{format_err, Result};
use std::{convert::TryFrom, sync::Arc, time::Duration};

mod in_memory;
pub use self::in_memory::*;

pub type Offset = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEvent {
    pub offset: Offset,
    pub details: Event,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithOffset<T> {
    pub offset: Offset,
    pub data: T,
}

pub trait Reader {
    fn get_start_offset(&self) -> Result<Offset>;

    fn read<'a>(
        &self,
        conn: &'a mut dyn Connection,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<WithOffset<Vec<LogEvent>>>;

    fn read_one<'a>(
        &self,
        conn: &'a mut dyn Connection,
        offset: Offset,
    ) -> Result<WithOffset<Option<LogEvent>>> {
        let WithOffset { offset, data } =
            self.read(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(data.len() <= 1);
        Ok(WithOffset {
            offset: offset,
            data: data.into_iter().next(),
        })
    }
}

pub trait Writer {
    fn write(&self, conn: &mut dyn Connection, events: &[Event]) -> Result<Offset> {
        self.write_tr(&mut *conn.start_transaction()?, events)
    }

    fn write_tr<'a>(&self, conn: &mut dyn Transaction<'a>, events: &[Event]) -> Result<Offset>;
}

pub type SharedReader = Arc<dyn Reader + Sync + Send + 'static>;
pub type SharedWriter = Arc<dyn Writer + Sync + Send + 'static>;
