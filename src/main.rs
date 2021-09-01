// Hey, it's not too bad https://blog.rust-lang.org/2021/08/03/GATs-stabilization-push.html
#![feature(generic_associated_types)]


mod auction;
mod event_log;
mod service;
mod persistence;

fn main() {
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let persistence = persistence::InMemoryPersistence{};
    let progress_store = service::progress::InMemoryProgressTracker::new_shared();
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let svc_ctr = service::ServiceControl::new();

    let _bidding_engine = service::bidding_engine::Service::new(
        &svc_ctr,
        persistence,
        progress_store.clone(),
        bidding_state_store,
        event_reader,
        event_writer,
    );
}
