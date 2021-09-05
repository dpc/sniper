use std::{sync::Arc, time::Duration};

use super::bidding_engine;
use crate::{
    auction::{Amount, BidDetails, ItemId, ItemIdRef},
    event_log,
};
use anyhow::Result;

use super::*;
use crate::persistence;

mod xmpp;
pub use self::xmpp::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub item: ItemId,
    pub event: EventDetails,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventDetails {
    Bid(BidDetails),
    Closed,
}

pub trait AuctionHouseClient {
    fn place_bid(&self, item_id: ItemIdRef, price: Amount) -> Result<()>;
    fn poll(&self, timeout: Option<Duration>) -> Result<Option<Event>>;
}

pub type SharedAuctionHouseClient = Arc<dyn AuctionHouseClient + Send + Sync + 'static>;

pub struct AuctionHouseSender {
    auction_house_client: SharedAuctionHouseClient,
}

impl AuctionHouseSender {
    pub fn new(auction_house_client: SharedAuctionHouseClient) -> Self {
        Self {
            auction_house_client,
        }
    }
}

impl LogFollowerService for AuctionHouseSender {
    fn get_log_progress_id(&self) -> String {
        "auction-house-sender".to_owned()
    }

    fn handle_event<'a>(
        &mut self,
        _transaction: &mut Transaction<'a>,
        event: event_log::EventDetails,
    ) -> Result<()> {
        match event {
            event_log::EventDetails::BiddingEngine(event) => match event {
                bidding_engine::Event::Bid(item_bid) => {
                    // Note: we rely on idempotency of this call to the server here
                    self.auction_house_client
                        .place_bid(&item_bid.item, item_bid.price)
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}

pub struct AuctionHouseReceiver {
    persistence: Persistence,
    even_writer: event_log::SharedWriter,
    auction_house_client: SharedAuctionHouseClient,
}

impl AuctionHouseReceiver {
    pub fn new(
        persistence: Persistence,
        even_writer: event_log::SharedWriter,
        auction_house_client: SharedAuctionHouseClient,
    ) -> Self {
        Self {
            persistence,
            auction_house_client,
            even_writer,
        }
    }
}

impl LoopService for AuctionHouseReceiver {
    fn run_iteration<'a>(&mut self) -> Result<()> {
        // TODO: no atomicity offered by the auction_house_client interface
        if let Some(event) = self
            .auction_house_client
            .poll(Some(Duration::from_secs(1)))?
        {
            let mut connection = self.persistence.get_connection()?;
            self.even_writer.write(
                &mut connection,
                &[event_log::EventDetails::AuctionHouse(event)],
            )?;
        }

        Ok(())
    }
}
