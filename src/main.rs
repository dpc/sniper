// Hey, it's not too bad https://blog.rust-lang.org/2021/08/03/GATs-stabilization-push.html
// See [persistence] module for why we need it.
#![feature(generic_associated_types)]
// since we're already on nightly...
#![feature(map_first_last)]

mod auction;
mod event_log;
mod persistence;
mod service;

fn main() {
    let persistence = persistence::InMemoryPersistence::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let progress_store = service::progress::InMemoryProgressTracker::new_shared();
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let svc_ctr = service::ServiceControl::new(progress_store);

    let _bidding_engine = service::bidding_engine::Service::new(
        &svc_ctr,
        persistence,
        bidding_state_store,
        event_reader,
        event_writer,
    );
}

#[cfg(test)]
mod tests;
