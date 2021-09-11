//! Bidding Engine
//!
//! The logic that based on events from the Ui and Auction House
//! determines if new bids should be created and of what amount.
use crate::{
    auction::{Amount, BidDetails, Bidder, ItemBid, ItemId, ItemIdRef},
    event::{AuctionHouseItemEvent, BiddingEngineAuctionError, BiddingEngineEvent, Event, UiEvent},
    event_log,
    persistence::{Connection, InMemoryTransaction, Transaction},
    service,
};
use anyhow::Result;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

mod postgres;

/// A store for the current state of each auction we participate in
pub trait BiddingStateStore {
    fn load_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        item_id: ItemIdRef,
    ) -> Result<Option<AuctionBiddingState>>;

    fn store_tr<'a>(
        &self,
        conn: &mut dyn Transaction<'a>,
        item_id: ItemIdRef,
        state: AuctionBiddingState,
    ) -> Result<()>;

    fn load(
        &self,
        conn: &mut dyn Connection,
        item_id: ItemIdRef,
    ) -> Result<Option<AuctionBiddingState>> {
        self.load_tr(&mut *conn.start_transaction()?, item_id)
    }

    fn store(
        &self,
        conn: &mut dyn Connection,
        item_id: ItemIdRef,
        state: AuctionBiddingState,
    ) -> Result<()> {
        self.store_tr(&mut *conn.start_transaction()?, item_id, state)
    }
}

pub type SharedBiddingStateStore = Arc<dyn BiddingStateStore + Send + Sync>;

pub struct InMemoryBiddingStateStore(Mutex<BTreeMap<ItemId, AuctionBiddingState>>);

impl InMemoryBiddingStateStore {
    pub fn new() -> Self {
        Self(Mutex::new(BTreeMap::default()))
    }

    pub fn new_shared() -> SharedBiddingStateStore {
        Arc::new(Self::new())
    }
}

impl BiddingStateStore for InMemoryBiddingStateStore {
    fn load_tr<'a>(
        &self,
        conn: &mut dyn Transaction,
        item_id: ItemIdRef,
    ) -> Result<Option<AuctionBiddingState>> {
        conn.cast().as_mut::<InMemoryTransaction>()?;
        Ok(self.0.lock().expect("lock").get(item_id).cloned())
    }

    fn store_tr<'a>(
        &self,
        conn: &mut dyn Transaction,
        item_id: ItemIdRef,
        state: AuctionBiddingState,
    ) -> Result<()> {
        conn.cast().as_mut::<InMemoryTransaction>()?;
        self.0
            .lock()
            .expect("lock")
            .insert(item_id.to_owned(), state);
        Ok(())
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub struct AuctionState {
    pub higest_bid: Option<BidDetails>,
    pub closed: bool,
}

impl AuctionState {
    pub fn handle_auction_event(mut self, event: AuctionHouseItemEvent) -> Self {
        match event {
            AuctionHouseItemEvent::Bid(bid) => {
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
            AuctionHouseItemEvent::Closed => {
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

#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct AuctionBiddingState {
    pub max_bid: Amount,
    pub state: AuctionState,
}

impl AuctionBiddingState {
    pub fn handle_auction_house_event(self, event: AuctionHouseItemEvent) -> Self {
        Self {
            max_bid: self.max_bid,
            state: self.state.handle_auction_event(event),
        }
    }
    pub fn handle_new_max_bid(self, max_bid: Amount) -> Self {
        Self { max_bid, ..self }
    }
}

pub const BIDDING_ENGINE_SERVICE_ID: &'static str = "bidding-engine";

pub struct BiddingEngine {
    bidding_state_store: SharedBiddingStateStore,
    even_writer: event_log::SharedWriter,
}

impl BiddingEngine {
    pub fn new(
        bidding_state_store: SharedBiddingStateStore,
        even_writer: event_log::SharedWriter,
    ) -> Self {
        Self {
            bidding_state_store,
            even_writer,
        }
    }

    fn handle_event_with<'a>(
        transaction: &mut dyn Transaction<'a>,
        bidding_state_store: &SharedBiddingStateStore,
        event_writer: &event_log::SharedWriter,
        item_id: ItemId,
        f: impl FnOnce(
            Option<AuctionBiddingState>,
        ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)>,
    ) -> Result<()> {
        let auction_state = bidding_state_store.load_tr(transaction, &item_id)?;

        let (new_state, events) = f(auction_state)?;

        if let Some(new_state) = new_state {
            bidding_state_store.store_tr(transaction, &item_id, new_state)?;
        }

        event_writer.write_tr(
            transaction,
            &events
                .into_iter()
                .map(|e| Event::BiddingEngine(e))
                .collect::<Vec<_>>(),
        )?;

        Ok(())
    }

    pub fn handle_auction_house_event(
        item_id: ItemId,
        old_state: Option<AuctionBiddingState>,
        event: AuctionHouseItemEvent,
    ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)> {
        Ok(if let Some(auction_state) = old_state {
            let new_state = auction_state.handle_auction_house_event(event);

            if new_state != auction_state {
                (
                    Some(new_state),
                    new_state
                        .state
                        .get_next_bid(new_state.max_bid)
                        .map(move |our_bid| {
                            BiddingEngineEvent::Bid(ItemBid {
                                item: item_id,
                                price: our_bid,
                            })
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (None, vec![])
            }
        } else {
            (
                None,
                vec![BiddingEngineEvent::AuctionError(
                    BiddingEngineAuctionError::UnknownAuction(item_id),
                )],
            )
        })
    }

    pub fn handle_max_bid_event(
        item_id: ItemId,
        old_state: Option<AuctionBiddingState>,
        price: Amount,
    ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)> {
        let auction_state = old_state.unwrap_or_else(Default::default);

        let new_state = auction_state.handle_new_max_bid(price);

        Ok(
            if new_state != auction_state
                && new_state
                    .state
                    .higest_bid
                    .map(|bid| bid.bidder != Bidder::Sniper)
                    .unwrap_or(true)
            {
                (
                    Some(new_state),
                    new_state
                        .state
                        .get_next_bid(new_state.max_bid)
                        .map(move |our_bid| {
                            BiddingEngineEvent::Bid(ItemBid {
                                item: item_id,
                                price: our_bid,
                            })
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (None, vec![])
            },
        )
    }
}

impl service::LogFollowerService for BiddingEngine {
    fn handle_event<'a>(
        &mut self,
        transaction: &mut dyn Transaction<'a>,
        event: Event,
    ) -> Result<()> {
        Ok(match event {
            Event::AuctionHouse(event) => Self::handle_event_with(
                transaction,
                &self.bidding_state_store,
                &self.even_writer,
                event.item.clone(),
                |old_state| Self::handle_auction_house_event(event.item, old_state, event.event),
            )?,
            Event::Ui(UiEvent::MaxBidSet(item_bid)) => Self::handle_event_with(
                transaction,
                &self.bidding_state_store,
                &self.even_writer,
                item_bid.item.clone(),
                |old_state| Self::handle_max_bid_event(item_bid.item, old_state, item_bid.price),
            )?,
            _ => (),
        })
    }

    fn get_log_progress_id(&self) -> String {
        BIDDING_ENGINE_SERVICE_ID.into()
    }
}
