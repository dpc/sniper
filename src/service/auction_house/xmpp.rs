use super::*;

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
    fn place_bid(&self, _item_id: ItemIdRef, _price: Amount) -> Result<()> {
        todo!()
    }

    fn poll(&self, timeout: Option<Duration>) -> Result<Option<Event>> {
        timeout.map(|t| std::thread::sleep(t));
        // TODO
        Ok(None)
    }
}
