use std::sync::Arc;
use std::time::Duration;

use crate::auction::{Amount, BidDetails, Bid, ItemIdRef};
use crate::event_log;
use anyhow::Result;

use super::JoinHandle;

pub enum Event {
    Bid(Bid),
    Lost,
    Won,
}

pub trait AuctionHouseClient {
    fn place_bid(&self, item_id: ItemIdRef, price: Amount) -> Result<()>;
    fn poll(&self, timeout: Option<Duration>) -> Result<Option<Event>>;
}

pub type SharedAuctionHouseClient = Arc<dyn AuctionHouseClient + Send + Sync + 'static>;

pub struct Service {
    reader_thread: JoinHandle,
    writer_thread: JoinHandle,
}

pub const WRITER_ID: &'static str = "auction-house-reader";

impl Service {
    fn new(
        svc_ctl: super::ServiceControl,
        progress_store: super::progress::SharedProgressTracker,
        event_reader: event_log::SharedReader,
        even_writer: event_log::SharedWriter,
        auction_house_client: SharedAuctionHouseClient,
    ) -> Self {
        let reader_thread = svc_ctl.spawn_loop(move || {
            if let Some(event) = auction_house_client.poll(Some(Duration::from_secs(1)))? {
                even_writer.write(
                    &[event_log::EventDetails::AuctionHouse(event)])?;
            }

            Ok(())
        });

        let writer_thread = svc_ctl.spawn_event_loop(
            progress_store.clone(),
            WRITER_ID,
            event_reader,
            move |event| todo!(),
        );

        Self {
            reader_thread,
            writer_thread,
        }
    }
}
