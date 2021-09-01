use std::sync::Arc;
use std::time::Duration;

use super::bidding_engine;
use crate::auction::{Amount, BidDetails, ItemId, ItemIdRef};
use crate::event_log;
use anyhow::Result;

use crate::persistence;
use super::JoinHandle;

pub struct Event {
    pub item: ItemId,
    pub event: EventDetails,
}

pub enum EventDetails {
    Bid(BidDetails),
    Closed,
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
    fn new<P>(
        svc_ctl: super::ServiceControl,
        persistence: P,
        progress_store: super::progress::SharedProgressTracker,
        event_reader: event_log::SharedReader<P>,
        even_writer: event_log::SharedWriter<P>,
        auction_house_client: SharedAuctionHouseClient,
    ) -> Self where P: persistence::Persistence + 'static {
        let reader_thread = svc_ctl.spawn_loop({
            let auction_house_client = auction_house_client.clone();
            move || {
                if let Some(event) = auction_house_client.poll(Some(Duration::from_secs(1)))? {
                    even_writer.write(&[event_log::EventDetails::AuctionHouse(event)])?;
                }

                Ok(())
            }
        });

        let writer_thread = svc_ctl.spawn_event_loop(
            persistence,
            progress_store.clone(),
            WRITER_ID,
            event_reader,
            move |transaction, event| match event {
                event_log::EventDetails::BiddingEngine(event) => match event {
                    bidding_engine::Event::Bid(item_bid) => {
                        auction_house_client.place_bid(&item_bid.item, item_bid.price)
                    }
                    _ => Ok(()),
                },
                _ => Ok(()),
            },
        );

        Self {
            reader_thread,
            writer_thread,
        }
    }
}
