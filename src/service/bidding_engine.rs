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
use tracing::{debug, span, Level};

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

/// Bidding state from a perspective of the auction house
///
/// Constructed from the events delivered from the (remote) Auction House.
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

    fn get_next_valid_bid(self, max_price: Amount) -> Option<Amount> {
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
                if dbg!(outbid_price) <= dbg!(max_price) {
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
    pub max_bid_limit: Amount,
    pub last_bid_sent: Option<Amount>,
    pub auction_state: AuctionState,
}

impl AuctionBiddingState {
    pub fn is_bid_better_than_last_bid_sent(self, amount: Amount) -> bool {
        self.last_bid_sent.is_none() || self.last_bid_sent.unwrap_or(0) < amount
    }

    pub fn handle_auction_house_event(self, event: AuctionHouseItemEvent) -> Self {
        Self {
            auction_state: self.auction_state.handle_auction_event(event),
            ..self
        }
    }
}

pub const BIDDING_ENGINE_SERVICE_ID: &'static str = "bidding-engine";

pub struct BiddingEngine {
    bidding_state_store: SharedBiddingStateStore,
    event_writer: event_log::SharedWriter,
}

impl BiddingEngine {
    pub fn new(
        bidding_state_store: SharedBiddingStateStore,
        event_writer: event_log::SharedWriter,
    ) -> Self {
        Self {
            bidding_state_store,
            event_writer,
        }
    }

    fn handle_auction_item_event_with<'a, T>(
        &self,
        transaction: &mut dyn Transaction<'a>,
        item_id: ItemIdRef,
        data: T,
        f: impl FnOnce(
            ItemIdRef,
            Option<AuctionBiddingState>,
            T,
        ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)>,
    ) -> Result<()> {
        let old_auction_state = self.bidding_state_store.load_tr(transaction, &item_id)?;

        let (new_auction_state, events) = f(item_id, old_auction_state, data)?;

        if let Some(new_state) = new_auction_state {
            if Some(new_state) != old_auction_state {
                self.bidding_state_store
                    .store_tr(transaction, item_id, new_state)?;
            }
        }

        debug!(?events, "write events");
        self.event_writer.write_tr(
            transaction,
            &events
                .into_iter()
                .map(|e| Event::BiddingEngine(e))
                .collect::<Vec<_>>(),
        )?;

        Ok(())
    }

    pub fn handle_auction_house_event(
        item_id: ItemIdRef,
        old_state: Option<AuctionBiddingState>,
        event: AuctionHouseItemEvent,
    ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)> {
        if let Some(auction_state) = old_state {
            Self::handle_next_bid_decision_for_new_state(
                item_id,
                auction_state.handle_auction_house_event(event),
            )
        } else {
            Ok((
                None,
                vec![BiddingEngineEvent::AuctionError(
                    BiddingEngineAuctionError::UnknownAuction(item_id.to_owned()),
                )],
            ))
        }
    }

    pub fn handle_max_bid_limit_event(
        item_id: ItemIdRef,
        old_state: Option<AuctionBiddingState>,
        price: Amount,
    ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)> {
        let old_state = old_state.unwrap_or_else(Default::default);

        Self::handle_next_bid_decision_for_new_state(
            item_id,
            AuctionBiddingState {
                max_bid_limit: price,
                ..old_state
            },
        )
    }

    pub fn handle_next_bid_decision_for_new_state(
        item_id: ItemIdRef,
        mut new_state: AuctionBiddingState,
    ) -> Result<(Option<AuctionBiddingState>, Vec<BiddingEngineEvent>)> {
        if let Some(our_new_bid) = new_state
            .auction_state
            .get_next_valid_bid(new_state.max_bid_limit)
        {
            if new_state.is_bid_better_than_last_bid_sent(our_new_bid) {
                new_state.last_bid_sent = Some(our_new_bid);

                Ok((
                    Some(new_state),
                    vec![BiddingEngineEvent::Bid(ItemBid {
                        item: item_id.to_owned(),
                        price: our_new_bid,
                    })],
                ))
            } else {
                Ok((Some(new_state), vec![]))
            }
        } else {
            Ok((Some(new_state), vec![]))
        }
    }
}

impl service::LogFollowerService for BiddingEngine {
    fn handle_event<'a>(
        &mut self,
        transaction: &mut dyn Transaction<'a>,
        event: Event,
    ) -> Result<()> {
        let span = span!(Level::DEBUG, "bidding engine - handle event");
        let _guard = span.enter();
        debug!(?event, "event");
        Ok(match event {
            Event::AuctionHouse(event) => self.handle_auction_item_event_with(
                transaction,
                &event.item,
                event.event,
                Self::handle_auction_house_event,
            )?,
            Event::Ui(UiEvent::MaxBidSet(item_bid)) => self.handle_auction_item_event_with(
                transaction,
                &item_bid.item,
                item_bid.price,
                Self::handle_max_bid_limit_event,
            )?,
            _ => (),
        })
    }

    fn get_log_progress_id(&self) -> String {
        BIDDING_ENGINE_SERVICE_ID.into()
    }
}
