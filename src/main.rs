mod auction;
mod event_log;
mod service;

fn main() {
    let (event_writer, event_reader) = event_log::new_in_memory_shared();
    let progress_store = service::progress::InMemoryProgressTracker::new_shared();
    let bidding_state_store = service::bidding_engine::InMemoryProgressTracker::new_shared();

    let svc_ctr = service::ServiceControl::new();

    let _bidding_engine = service::bidding_engine::Service::new(
        &svc_ctr,
        progress_store.clone(),
        bidding_state_store,
        event_reader,
        event_writer,
    );
}
