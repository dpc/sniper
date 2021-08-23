pub type ItemId = String;
pub type ItemIdRef<'s> = &'s str;
pub type Amount = u64;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Bidder {
    Sniper,
    Other,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Bid {
    pub item: ItemId,
    pub details: BidDetails,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ItemBid {
    pub item: ItemId,
    pub price: Amount,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BidDetails {
    pub bidder: Bidder,
    pub price: Amount,
    pub increment: Amount,
}

impl BidDetails {
    pub fn next_valid_bid(self) -> Amount {
        self.price + self.increment
    }

    pub fn is_outbidded_by(self, other: Amount) -> bool {
        self.next_valid_bid() <= other
    }
}
