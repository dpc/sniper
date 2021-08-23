//! Bidding Engine
//!
//! The logic that based on events from the Ui and Auction House
//! determines if new bids should be created and of what amount.
use super::JoinHandle;
use crate::auction::{Amount, BidDetails, Bidder, ItemBid, ItemId, ItemIdRef};
use crate::event_log;
use crate::service::{auction_house, ui};
use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// A store for the current state of each auction we participate in
pub trait BiddingStateStore {
    fn load(&self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>>;
    fn store(&self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()>;
}

pub type SharedBiddingStateStore = Arc<dyn BiddingStateStore + Send + Sync>;

pub struct InMemoryProgressTracker(Mutex<BTreeMap<ItemId, AuctionBiddingState>>);

impl InMemoryProgressTracker {
    pub fn new() -> Self {
        Self(Mutex::new(BTreeMap::default()))
    }

    pub fn new_shared() -> SharedBiddingStateStore {
        Arc::new(Self::new())
    }
}

impl BiddingStateStore for InMemoryProgressTracker {
    fn load(&self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>> {
        Ok(self.0.lock().expect("lock").get(item_id).cloned())
    }

    fn store(&self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()> {
        self.0
            .lock()
            .expect("lock")
            .insert(item_id.to_owned(), state);
        Ok(())
    }
}

#[derive(Error, Debug, Copy, Clone)]
pub enum UserError {
    #[error("auction already closed")]
    AlreadyClosed,
    #[error("bid is too low")]
    TooLow,
}

#[derive(Error, Debug, Clone)]
pub enum AuctionError {
    #[error("unknown auction: {0}")]
    UnknownAuction(ItemId),
}

#[derive(Clone)]
pub enum Event {
    /// We are placing a bid
    Bid(ItemBid),
    /// Auction house event caused an error
    AuctionError(AuctionError),
    /// User event caused an error
    UserError(UserError),
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct AuctionState {
    higest_bid: Option<BidDetails>,
    closed: bool,
}

impl AuctionState {
    pub fn handle_auction_event(mut self, event: auction_house::EventDetails) -> Self {
        match event {
            auction_house::EventDetails::Bid(bid) => {
                if !self.closed
                    && self
                        .higest_bid
                        .map(|highest| highest.is_outbidded_by(bid.price))
                        .unwrap_or(true)
                {
                    self.higest_bid = Some(bid);
                }
                self
            }
            auction_house::EventDetails::Closed => {
                self.closed = true;
                self
            }
        }
    }

    /*
    fn event(self, event: Event) -> Result<Self, Error> {
        use Event::*;
        Ok(match event {
            Bid(bid) => {
                self.ensure_valid_bid(bid)?;
                Self {
                    higest_bid: Some(bid),
                    ..self
                }
            }
            Closed => Self {
                closed: true,
                ..self
            },
        })
    }

    fn ensure_valid_bid(self, bid: BidDetails) -> Result<(), EventError> {
        use EventError::*;

        if self.closed {
            return Err(AlreadyClosed);
        }
        if let Some(highest_bid) = self.higest_bid {
            if !highest_bid.is_outbidded_by(bid.price) {
                return Err(TooLow);
            }
        }
        Ok(())
    }
    */

    fn get_next_bid(self, max_price: Amount) -> Option<Amount> {
        if self.closed {
            return None;
        }

        match self.higest_bid {
            // TODO: is 0 a valid bid? :)
            None => Some(0),

            // our bid is the higest already
            Some(BidDetails {
                bidder: Bidder::Sniper,
                ..
            }) => None,
            Some(highest_bid) => {
                let outbid_price = highest_bid.next_valid_bid();
                if outbid_price <= max_price {
                    Some(outbid_price)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq)]
pub struct AuctionBiddingState {
    max_bid: Amount,
    state: AuctionState,
}

impl AuctionBiddingState {
    pub fn handle_auction_event(self, event: auction_house::EventDetails) -> Self {
        Self {
            max_bid: self.max_bid,
            state: self.state.handle_auction_event(event),
        }
    }
    pub fn handle_new_max_bid(self, max_bid: Amount) -> Self {
        Self {
            max_bid: max_bid,
            ..self
        }
    }
}

/*
pub struct BiddingEngineShared {
    store: OwnedBiddingStateStore,
    auction_rpc: OwnedAuctionRpc,
    stop_rpc_thread: atomic::AtomicBool,
}


pub struct BiddingEngine {
    shared: Arc<BiddingEngineShared>,
    rpc_thread: thread::JoinHandle<()>,
}

impl BiddingEngine {
    fn new(store: OwnedBiddingStateStore, auction_rpc: OwnedAuctionRpc) -> Self {
        let shared = Arc::new(BiddingEngineShared {
            store,
            auction_rpc,
            stop_rpc_thread: atomic::AtomicBool::new(false),
        });

        let rpc_thread = thread::spawn({
            let shared = shared.clone();
            move || shared.run_rpc_thread()
        });

        Self { shared, rpc_thread }
    }

    pub fn bid_for_item(&self, item_id: ItemIdRef, max_price: Amount) -> Result<Option<Action>> {
        self.shared.bid_for_item(item_id, max_price)
    }
}

*/

pub const BIDDING_ENGINE_SERVICE_ID: &'static str = "bidding-engine";

pub struct Service {
    thread: JoinHandle,
}

impl Service {
    pub fn new(
        svc_ctl: &super::ServiceControl,
        progress_store: super::progress::SharedProgressTracker,
        bidding_state_store: SharedBiddingStateStore,
        event_reader: event_log::SharedReader,
        even_writer: event_log::SharedWriter,
    ) -> Self {
        let thread = svc_ctl.spawn_event_loop(
            progress_store.clone(),
            BIDDING_ENGINE_SERVICE_ID,
            event_reader,
            move |event_details| {
                Ok(match event_details {
                    event_log::EventDetails::AuctionHouse(event) => Self::handle_auction_event(
                        &bidding_state_store,
                        &even_writer,
                        event.item,
                        event.event,
                    )?,
                    event_log::EventDetails::Ui(ui::Event::MaxBidSet(item_bid)) => {
                        Self::handle_new_max_bid(
                            &bidding_state_store,
                            &even_writer,
                            item_bid.item,
                            item_bid.price,
                        )?
                    }
                    _ => (),
                })
            },
        );
        Self { thread }
    }

    fn handle_auction_event(
        bidding_state_store: &SharedBiddingStateStore,
        event_writer: &event_log::SharedWriter,
        item_id: ItemId,
        event: crate::service::auction_house::EventDetails,
    ) -> Result<()> {
        if let Some(auction_state) = bidding_state_store.load(&item_id)? {
            let new_state = auction_state.handle_auction_event(event);

            if new_state != auction_state {
                // TODO: wrap everything in a db transaction
                bidding_state_store.store(&item_id, auction_state)?;

                if let Some(our_bid) = new_state.state.get_next_bid(new_state.max_bid) {
                    event_writer.write(&[event_log::EventDetails::BiddingEngine(Event::Bid(
                        ItemBid {
                            item: item_id,
                            price: our_bid,
                        },
                    ))])?;
                }
            }
        } else {
            event_writer.write(
                &[event_log::EventDetails::BiddingEngine(Event::AuctionError(
                    AuctionError::UnknownAuction(item_id),
                ))],
            )?;
        }
        Ok(())
    }

    fn handle_new_max_bid(
        bidding_state_store: &SharedBiddingStateStore,
        event_writer: &event_log::SharedWriter,
        item_id: ItemId,
        price: Amount,
    ) -> Result<()> {
        let auction_state = bidding_state_store
            .load(&item_id)?
            .unwrap_or_else(Default::default);

        let new_state = auction_state.handle_new_max_bid(price);

        if new_state != auction_state
            && new_state
                .state
                .higest_bid
                .map(|bid| bid.bidder != Bidder::Sniper)
                .unwrap_or(true)
        {
            // TODO: wrap everything in a db transaction
            bidding_state_store.store(&item_id, auction_state)?;

            if let Some(our_bid) = new_state.state.get_next_bid(new_state.max_bid) {
                event_writer.write(&[event_log::EventDetails::BiddingEngine(Event::Bid(
                    ItemBid {
                        item: item_id,
                        price: our_bid,
                    },
                ))])?;
            }
        }

        Ok(())
    }
}
