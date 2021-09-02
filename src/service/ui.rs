use crate::auction::ItemBid;

#[derive(Clone, Debug)]
pub enum Event {
    MaxBidSet(ItemBid),
}
