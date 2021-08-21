use super::JoinHandle;
use crate::event_log;
use crate::auction::{ItemIdRef, BidDetails, Amount};
use thiserror::Error;

use std::sync::Arc;
use anyhow::Result;

pub trait BiddingStateStore {
    fn load(&self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>>;
    fn store(&self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()>;
}

pub type SharedBiddingStateStore = Arc<dyn BiddingStateStore + Send + Sync>;

#[derive(Error, Debug)]
pub enum EventError {
    #[error("auction already closed")]
    AlreadyClosed,
    #[error("bid is too low")]
    TooLow,
}

#[derive(Copy, Clone)]
pub enum Event {
    /// We are placing a bid
    Bid(BidDetails),
    UserError,
}

#[derive(Copy, Clone)]
pub struct AuctionBiddingState {
    max_bid: Amount,
    state: AuctionState,
}


#[derive(Default, Copy, Clone)]
pub struct AuctionState {
    higest_bid: Option<BidDetails>,
    closed: bool,
}

impl AuctionState {
    fn event(self, event: Event) -> Result<Self, EventError> {
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

    fn get_next_bid(self, max_price: Amount) -> Option<Amount> {
        if self.closed {
            return None;
        }

        match self.higest_bid {
            // TODO: is 0 a valid bid? :)
            None => Some(0),
            Some(highest_bid) => {
                let outbid_price = highest_bid.outbid_price();
                if outbid_price <= max_price {
                    Some(outbid_price)
                } else {
                    None
                }
            }
        }
    }
}

/*
pub struct BiddingEngineShared {
    store: OwnedBiddingStateStore,
    auction_rpc: OwnedAuctionRpc,
    stop_rpc_thread: atomic::AtomicBool,
}

impl BiddingEngineShared {
    pub fn handle_event(&self, event: RpcEvent) -> Result<Option<Action>> {
        todo!();
    }

    /// A background thread polling events from `auction_rpc` and calling
    /// `self` to handle them.
    fn run_rpc_thread(&self) {
        while !self.stop_rpc_thread.load(atomic::Ordering::SeqCst) {
            if let Err(e) = (|| -> Result<()> {
                let event = self.auction_rpc.poll_event()?;
                self.handle_event(event)?;
                Ok(())
            })() {
                todo!("handle errors");
            };
        }
    }

    pub fn bid_for_item(&self, item_id: ItemIdRef, max_price: Amount) -> Result<Option<Action>> {
        todo!();
    }
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

pub const BIDDING_ENGINE: &'static str = "bidding-engine";

struct Service {
    thread: JoinHandle,
}

impl Service {
    pub fn new(
            svc_ctl: super::ServiceControl,
            progress_store: super::progress::SharedProgressTracker,
            bidding_state_store : SharedBiddingStateStore,
            event_reader: event_log::SharedReader,
            even_writer: event_log::SharedWriter,
        ) -> Self {
            let thread = svc_ctl.spawn_event_loop(
                progress_store.clone(),
                BIDDING_ENGINE,
                event_reader,
                move |event_details| Ok(match event_details {
                    event_log::EventDetails::AuctionHouse(bid) => todo!(),
                    event_log::EventDetails::Ui => todo!(),
                    _ => (),

                }),
            );

            Self {
                thread
            }
    }

}
