use crate::{
    auction, event_log,
    persistence::{self, Persistence},
    service,
};
use anyhow::Result;

#[test]
fn sends_a_bid_when_asked_to() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let progress_store = service::progress::InMemoryProgressTracker::new_shared();
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let svc_ctr = service::ServiceControl::new(progress_store);

    let _bidding_engine = service::bidding_engine::Service::new(
        &svc_ctr,
        persistence.clone(),
        bidding_state_store,
        event_reader,
        event_writer.clone(),
    );

    let mut conn = persistence.get_connection()?;
    event_writer.write(
        &mut conn,
        &[event_log::EventDetails::Ui(service::ui::Event::MaxBidSet(
            auction::ItemBid {
                item: "foo".to_owned(),
                price: 100,
            },
        ))],
    )?;

    todo!();
}
