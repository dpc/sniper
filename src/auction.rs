use anyhow::Result;
use std::sync::{atomic, Arc};
use std::thread;
use thiserror::Error;

pub type ItemId = String;
pub type ItemIdRef<'s> = &'s str;
pub type Amount = u64;

#[derive(Error, Debug)]
pub enum EventError {
    #[error("auction already closed")]
    AlreadyClosed,
    #[error("bid is too low")]
    TooLow,
}

pub enum Action {
    Bid(Amount),
    Join,
}

#[derive(Copy, Clone)]
pub enum Bidder {
    Sniper,
    Other,
}

#[derive(Copy, Clone)]
pub struct Bid {
    bidder: Bidder,
    price: Amount,
    increment: Amount,
}

impl Bid {
    fn outbid_price(self) -> Amount {
        self.price + self.increment
    }

    fn is_outbidded_by(self, other: Amount) -> bool {
        self.outbid_price() <= other
    }
}

#[derive(Default, Copy, Clone)]
pub struct AuctionState {
    higest_bid: Option<Bid>,
    closed: bool,
}

#[derive(Copy, Clone)]
pub struct AuctionBiddingState {
    max_bid: Amount,
    state: AuctionState,
}

#[derive(Copy, Clone)]
pub enum Event {
    Bid(Bid),
    Closed,
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

    fn ensure_valid_bid(self, bid: Bid) -> Result<(), EventError> {
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

    fn get_action(self, max_price: Amount) -> Option<Action> {
        if self.closed {
            return None;
        }

        match self.higest_bid {
            // TODO: is 0 a valid bid? :)
            None => Some(Action::Bid(0)),
            Some(highest_bid) => {
                let outbid_price = highest_bid.outbid_price();
                if outbid_price <= max_price {
                    Some(Action::Bid(outbid_price))
                } else {
                    None
                }
            }
        }
    }
}

pub trait BiddingStateStore {
    fn load(&self, item_id: ItemIdRef) -> Result<Option<AuctionBiddingState>>;
    fn store(&self, item_id: ItemIdRef, state: AuctionBiddingState) -> Result<()>;
}

pub type OwnedBiddingStateStore = Box<dyn BiddingStateStore + Send + Sync>;

pub struct RpcEvent {}

pub trait AuctionRpc {
    fn send_bid(&self, item: ItemIdRef, bid: Amount) -> Result<()>;
    fn poll_event(&self) -> Result<RpcEvent>;
}

pub type OwnedAuctionRpc = Box<dyn AuctionRpc + Send + Sync>;
pub type SharedAuctionRpc = Arc<dyn AuctionRpc + Send + Sync>;

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
