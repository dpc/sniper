use super::*;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct XmppAuctionHouseClient;

impl XmppAuctionHouseClient {
    pub fn new() -> Self {
        Self
    }

    pub fn new_shared() -> SharedAuctionHouseClient {
        Arc::new(Self::new())
    }
}

impl AuctionHouseClient for XmppAuctionHouseClient {
    fn place_bid(&self, item_id: ItemIdRef, price: Amount) -> Result<()> {
        debug!(?item_id, ?price, "sending bid");
        todo!()
    }

    fn poll(&self, timeout: Option<Duration>) -> Result<Option<AuctionHouseEvent>> {
        timeout.map(|t| std::thread::sleep(t));
        // TODO
        Ok(None)
    }
}
