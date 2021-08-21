pub type ItemId = String;
pub type ItemIdRef<'s> = &'s str;
pub type Amount = u64;

#[derive(Copy, Clone)]
pub enum Bidder {
    Sniper,
    Other,
}

pub struct Bid {
    pub item: ItemId,
    pub details: BidDetails,

}
#[derive(Copy, Clone)]
pub struct BidDetails {
    pub bidder: Bidder,
    pub price: Amount,
    pub increment: Amount,
}

impl BidDetails {
    pub fn outbid_price(self) -> Amount {
        self.price + self.increment
    }

    pub fn is_outbidded_by(self, other: Amount) -> bool {
        self.outbid_price() <= other
    }
}
