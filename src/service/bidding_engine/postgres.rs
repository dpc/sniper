use crate::persistence;
use std::convert::TryFrom;
use anyhow::Result;

pub struct PostgresBiddingStateStore {
    client: postgres::Client,
}

impl super::BiddingStateStore for PostgresBiddingStateStore {
    type Persistence = persistence::postgres::PostgresPersistence;

    fn load(
        &self,
        conn: &mut persistence::postgres::PostgresConnection,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<super::AuctionBiddingState>> {
        Ok(
            conn.client.query_opt("SELECT max_bid, higest_bid_bidder, higest_bid_price, highest_bid_increment, closed FROM bidding_state", &[])?
            .map::<Result<_>, _>(|row| {
            Ok(super::AuctionBiddingState {
                max_bid: u64::try_from(row.get::<'_, _, i64>("max_bid"))?,
                state: super::AuctionState {
                    closed: row.get("closed"),
                    higest_bid: todo!(),
                }
            })
        }).transpose()?)
    }

    fn store(
        &self,
        conn: &mut persistence::postgres::PostgresConnection,
        item_id: crate::auction::ItemIdRef,
        state: super::AuctionBiddingState,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn load_tr<'a>(
        &self,
        conn: &mut persistence::postgres::PostgresTransaction<'a>,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<super::AuctionBiddingState>> {
        todo!()
    }

    fn store_tr<'a>(
        &self,
        conn: &mut persistence::postgres::PostgresTransaction<'a>,
        item_id: crate::auction::ItemIdRef,
        state: super::AuctionBiddingState,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
