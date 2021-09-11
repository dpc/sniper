use super::*;
use crate::{event::Event, persistence::InMemoryTransaction};

type InMemoryLogInner = Vec<Event>;

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

    fn write_events(&self, events: &[Event]) -> Result<Offset> {
        let mut write = self.write();

        write.extend_from_slice(events);
        self.condvar.notify_all();

        Ok(u64::try_from(write.len())?)
    }
}

impl Reader for InMemoryLog {
    fn read_tr<'a>(
        &self,
        _conn: &mut dyn Transaction,
        offset: Offset,
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Offset, Vec<LogEvent>)> {
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
            .map(|(i, e)| LogEvent {
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
    fn write_tr<'a>(&self, conn: &mut dyn Transaction, events: &[Event]) -> Result<Offset> {
        conn.cast().as_mut::<InMemoryTransaction>()?;
        self.write_events(events)
    }
}

pub fn new_in_memory_shared() -> (SharedWriter, SharedReader) {
    let log = Arc::new(InMemoryLog {
        inner: RwLock::new(Vec::new()),
        condvar: util::CondvarAny::default(),
    });
    (log.clone(), log)
}
