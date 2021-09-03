use crate::{
    auction,
    auction::ItemBid,
    event_log,
    persistence::{self, Connection, Persistence, Transaction},
    service,
    service::{bidding_engine::*, Service},
};
use anyhow::Result;
use std::time::Duration;

#[test]
fn sends_a_bid_when_asked_to() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let mut bidding_engine =
        service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer);

    bidding_engine.handle_event(
        &mut persistence.get_connection()?.start_transaction()?,
        event_log::EventDetails::Ui(service::ui::Event::MaxBidSet(auction::ItemBid {
            item: "foo".to_owned(),
            price: 100,
        })),
    )?;

    assert_eq!(
        event_reader
            .read(
                &mut persistence.get_connection()?,
                event_reader.get_start_offset()?,
                1,
                Some(Duration::from_secs(0))
            )?
            .1
            .iter()
            .map(|e| e.details.clone())
            .collect::<Vec<_>>(),
        vec![event_log::EventDetails::BiddingEngine(Event::Bid(
            ItemBid {
                item: "foo".to_owned(),
                price: 0
            }
        ))]
    );

    Ok(())
}
