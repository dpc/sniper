mod auction;
mod event;
mod event_log;
mod persistence;
mod progress;
mod service;

use anyhow::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let persistence = Arc::new(persistence::InMemoryPersistence::new());
    let progress_store = progress::InMemoryProgressTracker::new_shared();
    let (event_writer, event_reader) = event_log::new_in_memory_shared()?;
    let auction_house_client = service::auction_house::XmppAuctionHouseClient::new_shared();

    let svc_ctr = service::ServiceControl::new(persistence.clone(), progress_store);

    ctrlc::set_handler({
        let svc_ctr = svc_ctr.clone();
        move || {
            eprintln!("Stopping all services...");
            svc_ctr.send_stop_to_all();
        }
    })?;

    let bidding_state_store = service::InMemoryBiddingStateStore::new_shared();
    for handle in vec![
        svc_ctr.spawn_log_follower(
            service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer.clone()),
            event_reader.clone(),
        ),
        svc_ctr.spawn_loop(service::AuctionHouseReceiver::new(
            persistence.clone(),
            event_writer.clone(),
            auction_house_client.clone(),
        )),
        svc_ctr.spawn_log_follower(
            service::AuctionHouseSender::new(auction_house_client.clone()),
            event_reader.clone(),
        ),
        svc_ctr.spawn_loop(service::Ui::new(persistence, event_writer.clone())?),
    ] {
        handle.join()?
    }

    Ok(())
}

#[cfg(test)]
mod tests;
