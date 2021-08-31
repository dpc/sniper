pub struct PostgresBiddingStateStore {
    client: postgres::Client,
}

impl super::BiddingStateStore for PostgresBiddingStateStore {
    fn load(&self, item_id: crate::auction::ItemIdRef) -> anyhow::Result<Option<super::AuctionBiddingState>> {
        todo!()
    }

    fn store(&self, item_id: crate::auction::ItemIdRef, state: super::AuctionBiddingState) -> anyhow::Result<()> {
        todo!()
    }
}
