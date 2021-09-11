use crate::{
    event::Event,
    persistence::{Connection, Transaction},
};
use anyhow::{format_err, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{convert::TryFrom, sync::Arc, time::Duration};

mod in_memory;
pub use self::in_memory::*;

pub type Offset = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEvent {
    pub offset: Offset,
    pub details: Event,
}

pub trait Reader {
    fn get_start_offset(&self) -> Result<Offset>;

    fn read_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<LogEvent>)>;

    fn read<'a>(
        &self,
        conn: &mut dyn Connection,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<LogEvent>)> {
        self.read_tr(&mut *conn.start_transaction()?, offset, limit, timeout)
    }

    fn read_one_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        offset: Offset,
    ) -> Result<(Offset, Option<LogEvent>)> {
        let (offset, v) = self.read_tr(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
    }

    fn read_one<'a>(
        &self,
        conn: &mut dyn Connection,
        offset: Offset,
    ) -> Result<(Offset, Option<LogEvent>)> {
        let (offset, v) = self.read(conn, offset, 1, Some(Duration::from_millis(0)))?;
        assert!(v.len() <= 1);
        Ok((offset, v.into_iter().next()))
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
