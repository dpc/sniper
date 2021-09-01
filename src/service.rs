pub mod auction_house;
pub mod bidding_engine;
pub mod progress;
pub mod ui;

use crate::persistence;
use crate::persistence::Connection;
use crate::persistence::Transaction;
use anyhow::format_err;
use anyhow::Result;
use std::sync::{
    atomic::{self, AtomicBool},
    Arc,
};
use std::thread;

use crate::event_log;

use self::progress::SharedProgressTracker;

pub type ServiceId = String;
pub type ServiceIdRef<'a> = &'a str;

/// An utility control structure to control service execution
///
/// All services are basically a loop, and we would like to be able to
/// gracefully terminate them, and handle and top-level error of any
/// of them by stopping everything.
#[derive(Default, Clone)]
pub struct ServiceControl {
    stop: Arc<AtomicBool>,
}

impl ServiceControl {
    pub fn new() -> Self {
        Default::default()
    }

    /// Start a new service as a loop, with a certain body
    ///
    /// This will take care of checking termination condition and
    /// handling any errors returned by `f`
    pub fn spawn_loop<F>(&self, mut f: F) -> JoinHandle
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        JoinHandle::new(thread::spawn({
            let stop = self.stop.clone();
            move || {
                while !stop.load(atomic::Ordering::SeqCst) {
                    if let Err(e) = f() {
                        stop.store(true, atomic::Ordering::SeqCst);
                        return Err(e);
                    }
                }
                Ok(())
            }
        }))
    }

    pub fn spawn_event_loop<F, P>(
        &self,
        persistence: P,
        progress_store: SharedProgressTracker,
        service_id: ServiceIdRef,
        event_reader: event_log::SharedReader<P>,
        mut f: F,
    ) -> JoinHandle
    where
        F: for <'a> FnMut(&mut <<P as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>, event_log::EventDetails) -> Result<()> + Send + Sync + 'static,
        P: persistence::Persistence + 'static,
    {
        let service_id = service_id.to_owned();

        let mut progress = match progress_store.load(&service_id) {
            Err(e) => return JoinHandle::new(thread::spawn(move || Err(e))),
            Ok(o) => o,
        };

        self.spawn_loop(move || {

            let mut connection = persistence.get_connection()?;
            let mut transaction = connection.start_transaction()?;

            for event in event_reader
                .read(progress.clone(), 1, Some(std::time::Duration::from_secs(1)))?
                .drain(..)
            {
                f(&mut transaction, event.details)?;

                progress = Some(event.id.clone());
                progress_store.store(&service_id, &event.id)?;
            }
            transaction.commit()?;
            Ok(())
        })
    }
}

/// Simple thread join wrapper that joins the thread on drop
///
/// TODO: Would it be better to have it set the `stop` flag toc terminate all threads
/// on drop?
pub struct JoinHandle(Option<thread::JoinHandle<Result<()>>>);

impl JoinHandle {
    fn new(handle: thread::JoinHandle<Result<()>>) -> Self {
        JoinHandle(Some(handle))
    }
}

impl JoinHandle {
    fn join_mut(&mut self) -> Result<()> {
        if let Some(h) = self.0.take() {
            h.join().map_err(|e| format_err!("join failed: {:?}", e))?
        } else {
            Ok(())
        }
    }

    pub fn join(mut self) -> Result<()> {
        self.join_mut()
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.join_mut().expect("not failed")
    }
}
