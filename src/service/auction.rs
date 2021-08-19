use crate::auction::{Amount, Bid, ItemIdRef};
use crate::event_log;
use anyhow::Result;

use super::JoinHandle;

pub trait AuctionHouseAPI {
    fn place_bid(&self, item_id: ItemIdRef, price: Amount) -> Result<()>;
    fn poll(&self) -> Result<Bid>;
}

pub struct Handler {
    reader_thread: JoinHandle,
    writer_thread: JoinHandle,
}

impl Handler {
    fn new(
        svc_ctl: super::ServiceControl,
        event_reader: event_log::SharedReader,
        even_writer: event_log::SharedWriter,
    ) -> Self {
        let reader_thread = svc_ctl.spawn_loop(move || match event_reader.read(None, 1, None)? {
            _ => todo!(),
        });

        let writer_thread = svc_ctl.spawn_loop(|| {
            todo!();
        });
        Self {
            reader_thread,
            writer_thread,
        }
    }
}
