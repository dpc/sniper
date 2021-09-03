use crate::auction::ItemBid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    MaxBidSet(ItemBid),
}
