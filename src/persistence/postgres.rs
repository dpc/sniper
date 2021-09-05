use crate::service::bidding_engine::AuctionState;
use std::convert::TryFrom;

use super::*;

pub fn new(
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
) -> Persistence {
    Persistence(Arc::new(PostgresPersistence { pool }))
}

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl PersistenceImpl for PostgresPersistence {
    fn get_connection(self: Arc<Self>) -> Result<Connection> {
        let res = Connection(Box::new(self.pool.get()?));
        Ok(res)
    }
}

pub type PostgresConnection = r2d2::PooledConnection<
    r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>,
>;

impl ConnectionImpl for PostgresConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Transaction<'a>> {
        let res = Transaction(Box::new(self.transaction()?));
        Ok(res)
    }
}

pub type PostgresTransaction<'a> = ::postgres::Transaction<'a>;

impl<'a> TransactionImpl<'a> for PostgresTransaction<'a> {
    fn commit(self: Box<Self>) -> Result<()> {
        Ok(::postgres::Transaction::commit(*self)?)
    }

    fn rollback(self: Box<Self>) -> Result<()> {
        Ok(::postgres::Transaction::rollback(*self)?)
    }

    #[allow(unreachable_code)]
    fn load_tr(
        &mut self,
        item_id: crate::auction::ItemIdRef,
    ) -> anyhow::Result<Option<AuctionBiddingState>> {
        Ok(
            self.query_opt("SELECT max_bid, higest_bid_bidder, higest_bid_price, highest_bid_increment, closed FROM bidding_state WHERE item_id = $0", &[&item_id])?
            .map::<Result<_>, _>(|row| -> Result<AuctionBiddingState, anyhow::Error> {
            Ok(super::AuctionBiddingState {
                max_bid: u64::try_from(row.get::<'_, _, i64>("max_bid"))?,
                state: AuctionState {
                    closed: row.get("closed"),
                    higest_bid: todo!(),
                }
            })
        }).transpose()?)
    }

    fn store_tr(
        &mut self,
        _item_id: crate::auction::ItemIdRef,
        _state: AuctionBiddingState,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
