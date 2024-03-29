use crate::persistence::{
    postgres::PostgresTransaction, Connection, PostgresConnection, Transaction,
};
use anyhow::Result;
use std::convert::TryFrom;

pub struct PostgresBiddingStateStore {
    client: postgres::Client,
}

impl super::BiddingStateStore for PostgresBiddingStateStore {
    #[allow(unreachable_code)]
    fn load_tr(
        &self,
        conn: &mut dyn Transaction,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<super::AuctionBiddingState>> {
        conn.cast().as_mut::<PostgresTransaction>()?.0.query_opt("SELECT max_bid_limit, last_bid_sent, higest_bid_bidder, higest_bid_price, highest_bid_increment, closed FROM bidding_state WHERE item_id = $0", &[&item_id])?
            .map::<Result<_>, _>(|row| {
            Ok(super::AuctionBiddingState {
                max_bid_limit: u64::try_from(row.get::<'_, _, i64>("max_bid_limit"))?,
                last_bid_sent: row.get::<'_,_, Option<i64>>("last_bid_sent").map(u64::try_from).transpose()?,
                auction_state: super::AuctionState {
                    closed: row.get("closed"),
                    higest_bid: todo!(),
                }
            })
        }).transpose()
    }

    #[allow(unreachable_code)]
    fn load(
        &self,
        conn: &mut dyn Connection,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<super::AuctionBiddingState>> {
        conn.cast().as_mut::<PostgresConnection>()?.0.query_opt("SELECT max_bid_limit, last_bid_sent, higest_bid_bidder, higest_bid_price, highest_bid_increment, closed FROM bidding_state WHERE item_id = $0", &[&item_id])?
            .map::<Result<_>, _>(|row| {
            Ok(super::AuctionBiddingState {
                max_bid_limit: u64::try_from(row.get::<'_, _, i64>("max_bid_limit"))?,
                last_bid_sent: row.get::<'_,_, Option<i64>>("last_bid_sent").map(u64::try_from).transpose()?,
                auction_state: super::AuctionState {
                    closed: row.get("closed"),
                    higest_bid: todo!(),
                }
            })
        }).transpose()
    }
    fn store_tr(
        &self,
        _conn: &mut dyn Transaction,
        _item_id: crate::auction::ItemIdRef,
        _state: super::AuctionBiddingState,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
