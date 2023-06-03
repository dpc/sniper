mod in_memory;

pub use self::in_memory::*;

use crate::{
    event_log::Offset,
    persistence::{Connection, Transaction},
    service::{ServiceId, ServiceIdRef},
};
use anyhow::format_err;
use std::sync::Arc;

use anyhow::Result;

/// A persistent store to keep track of the last processed event
pub trait ProgressTracker {
    fn load(&self, conn: &mut dyn Connection, id: ServiceIdRef) -> Result<Option<Offset>>;

    fn store_tr(
        &self,
        conn: &mut dyn Transaction<'_>,
        id: ServiceIdRef,
        offset: Offset,
    ) -> Result<()>;
    fn load_tr(&self, conn: &mut dyn Transaction<'_>, id: ServiceIdRef) -> Result<Option<Offset>>;
}

pub type SharedProgressTracker = Arc<dyn ProgressTracker + Send + Sync + 'static>;
