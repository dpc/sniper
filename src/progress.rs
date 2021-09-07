mod in_memory;

pub use self::in_memory::*;

use crate::{
    event_log::Offset,
    persistence,
    service::{ServiceId, ServiceIdRef},
};
use anyhow::format_err;
use std::sync::Arc;

use anyhow::Result;

/// A persistent store to keep track of the last processed event
pub trait ProgressTracker {
    type Persistence: persistence::Persistence;
    fn load(
        &self,
        conn: &mut <<Self as ProgressTracker>::Persistence as persistence::Persistence>::Connection,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>>;

    fn store_tr<'a>(
        &self,
        conn: &mut <<Self as ProgressTracker>::Persistence as persistence::Persistence>::Transaction<'a>,
        id: ServiceIdRef,
        offset: Offset,
    ) -> Result<()>;
    fn load_tr<'a>(
        &self,
        conn: &mut <<Self as ProgressTracker>::Persistence as persistence::Persistence>::Transaction<'a>,
        id: ServiceIdRef,
    ) -> Result<Option<Offset>>;
}

pub type SharedProgressTracker<P> =
    Arc<dyn ProgressTracker<Persistence = P> + Send + Sync + 'static>;
