// Hey, it's not too bad https://blog.rust-lang.org/2021/08/03/GATs-stabilization-push.html
// See [persistence] module for why we need it.
#![feature(generic_associated_types)]
// since we're already on nightly...
#![feature(map_first_last)]

mod auction;
mod event_log;
mod persistence;
mod progress;
mod service;

use anyhow::Result;

fn main() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let progress_store = progress::InMemoryProgressTracker::new_shared();
    let auction_house_client = service::auction_house::XmppAuctionHouseClient::new_shared();

    let svc_ctr = service::ServiceControl::new(persistence.clone(), progress_store);

    ctrlc::set_handler({
        let svc_ctr = svc_ctr.clone();
        move || {
            eprintln!("Stopping all services...");
            svc_ctr.stop_all();
        }
    })?;

    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();
    for handle in vec![
        svc_ctr.spawn_log_follower(
            service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer.clone()),
            event_reader.clone(),
        ),
        svc_ctr.spawn_loop(service::auction_house::AuctionHouseReceiver::new(
            persistence.clone(),
            event_writer.clone(),
            auction_house_client.clone(),
        )),
        svc_ctr.spawn_log_follower(
            service::auction_house::AuctionHouseSender::new(auction_house_client.clone()),
            event_reader.clone(),
        ),
    ] {
        handle.join()?
    }

    Ok(())
}

#[cfg(test)]
mod tests;
