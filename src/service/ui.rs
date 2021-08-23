use crate::auction::ItemBid;

pub enum Event {
    MaxBidSet(ItemBid),
}
