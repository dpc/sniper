// Hey, it's not too bad https://blog.rust-lang.org/2021/08/03/GATs-stabilization-push.html
// See [persistence] module for why we need it.
#![feature(generic_associated_types)]
// since we're already on nightly...
#![feature(map_first_last)]

mod auction;
mod event_log;
mod persistence;
mod service;

use anyhow::Result;

fn main() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let progress_store = service::progress::InMemoryProgressTracker::new_shared();

    let svc_ctr = service::ServiceControl::new(persistence, progress_store);

    ctrlc::set_handler({
        let svc_ctr = svc_ctr.clone();
        move || {
            eprintln!("Stopping all services...");
            svc_ctr.stop_all();
        }
    })?;

    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();
    let bidding_engine = svc_ctr.spawn(
        service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer),
        event_reader,
    );

    bidding_engine.join()?;

    Ok(())
}

#[cfg(test)]
mod tests;
