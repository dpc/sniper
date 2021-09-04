use crate::{
    auction,
    auction::{Amount, BidDetails, Bidder, ItemBid, ItemIdRef},
    event_log,
    persistence::{self, Connection, Persistence},
    service,
    service::{bidding_engine::*, Service},
};
use anyhow::Result;

trait BiddingEngineTestExt<P>
where
    P: Persistence,
{
    fn handle_max_bid_event(
        &mut self,
        conn: &mut <P as Persistence>::Connection,
        id: ItemIdRef,
        price: Amount,
    ) -> Result<()>;
}

impl<P> BiddingEngineTestExt<P> for BiddingEngine<P>
where
    P: Persistence + 'static,
{
    fn handle_max_bid_event<'a>(
        &mut self,
        conn: &mut <P as Persistence>::Connection,
        id: ItemIdRef,
        price: Amount,
    ) -> Result<()> {
        self.handle_event(
            &mut conn.start_transaction()?,
            event_log::EventDetails::Ui(service::ui::Event::MaxBidSet(auction::ItemBid {
                item: id.to_owned(),
                price,
            })),
        )
    }
}

#[test]
fn sanity_check_sends_a_bid_when_asked_to_via_event_log() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let mut conn = persistence.get_connection()?;

    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let mut bidding_engine =
        service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer);

    bidding_engine.handle_max_bid_event(&mut conn, "foo", 100)?;

    let res = event_reader.read_one(&mut conn, event_reader.get_start_offset()?)?;

    assert_eq!(
        res.clone().1.map(|e| e.details),
        Some(event_log::EventDetails::BiddingEngine(Event::Bid(
            ItemBid {
                item: "foo".to_owned(),
                price: 0
            }
        )))
    );

    let res = event_reader.read_one(&mut conn, res.0)?;
    assert_eq!(res.1.map(|e| e.details), None);

    // sending the same bid again makes no difference
    bidding_engine.handle_max_bid_event(&mut conn, "foo", 100)?;

    let res = event_reader.read_one(&mut conn, res.0)?;
    assert_eq!(res.1.map(|e| e.details), None);
    Ok(())
}

#[test]
fn sends_a_bid_when_asked_to() -> Result<()> {
    /*
        let persistence = persistence::InMemoryPersistence::new();
        let mut conn = persistence.get_connection()?;

        let (event_writer, event_reader) = event_log::new_in_memory_shared();
        let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

        let mut bidding_engine =
            service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer);
    */
    assert_eq!(
        BiddingEngine::<()>::handle_max_bid_event("foo".to_string(), None, 100)?,
        (
            Some(AuctionBiddingState {
                max_bid: 100,
                state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Sniper,
                        price: 0,
                        increment: 0,
                    }),
                    closed: false
                },
            }),
            vec![Event::Bid(ItemBid {
                item: "foo".to_string(),
                price: 0
            })]
        )
    );

    Ok(())
}
