use crate::auction::*;
use thiserror::Error;

// TODO: This type makes everything cyclical:
// All services depend on it, and it depends
// on events of each of the services. Not a
// big deal for this small program, but something
// to take care of in a more realistic implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    AuctionHouse(AuctionHouseEvent),
    BiddingEngine(BiddingEngineEvent),
    Ui(UiEvent),
    #[cfg(test)]
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuctionHouseEvent {
    pub item: ItemId,
    pub event: AuctionHouseItemEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuctionHouseItemEvent {
    Bid(BidDetails),
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BiddingEngineEvent {
    /// We are placing a bid
    Bid(ItemBid),
    /// Auction house event caused an error
    AuctionError(BiddingEngineAuctionError),
    /// User event caused an error
    UserError(BiddingEngineUserError),
}

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BiddingEngineUserError {
    #[error("auction already closed")]
    AlreadyClosed,
    #[error("bid is too low")]
    TooLow,
}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum BiddingEngineAuctionError {
    #[error("unknown auction: {0}")]
    UnknownAuction(ItemId),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    MaxBidSet(ItemBid),
}
