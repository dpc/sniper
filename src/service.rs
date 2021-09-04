pub mod auction_house;
pub mod bidding_engine;
pub mod ui;

use crate::{
    persistence,
    persistence::{Connection, Transaction},
    progress,
};
use anyhow::{format_err, Result};
use std::{
    sync::{
        atomic::{self, AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use crate::event_log;

pub type ServiceId = String;
pub type ServiceIdRef<'a> = &'a str;

/// A service that handles events on the log
pub trait LogFollowerService<P>: Send + Sync
where
    P: persistence::Persistence + 'static,
{
    fn get_log_progress_id(&self) -> String;

    fn handle_event<'a>(
        &mut self,
        transaction: &mut <<P as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>,
        event: event_log::EventDetails,
    ) -> Result<()>;
}

/// A service that is a loop that does something
pub trait LoopService: Send + Sync {
    fn run_iteration<'a>(&mut self) -> Result<()>;
}

/// Service execution control instance
///
/// All services are basically a loop, and we would like to be able to
/// gracefully terminate them, and handle and top-level error of any
/// of them by gracefully stopping everything else.
#[derive(Clone)]
pub struct ServiceControl<P> {
    stop_all: Arc<AtomicBool>,
    progress_store: progress::SharedProgressTracker<P>,
    persistence: P,
}

impl<P> ServiceControl<P>
where
    P: persistence::Persistence + 'static,
{
    pub fn new(persistence: P, progress_store: progress::SharedProgressTracker<P>) -> Self {
        Self {
            stop_all: Default::default(),
            progress_store,
            persistence,
        }
    }

    pub fn stop_all(&self) {
        self.stop_all.store(true, Ordering::SeqCst);
    }

    pub fn spawn_log_follower(
        &self,
        mut service: impl LogFollowerService<P> + 'static,
        event_reader: event_log::SharedReader<P>,
    ) -> JoinHandle {
        self.spawn_event_loop(
            &service.get_log_progress_id(),
            event_reader,
            move |transaction, event_details| service.handle_event(transaction, event_details),
        )
    }

    pub fn spawn_loop(&self, mut service: impl LoopService + 'static) -> JoinHandle {
        self.spawn_loop_raw(move || service.run_iteration())
    }

    /// Start a new service as a loop, with a certain body
    ///
    /// This will take care of checking termination condition and
    /// handling any errors returned by `f`
    fn spawn_loop_raw<F>(&self, mut f: F) -> JoinHandle
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));

        JoinHandle::new(
            stop.clone(),
            thread::spawn({
                let stop_all = self.stop_all.clone();
                move || {
                    while !stop.load(atomic::Ordering::SeqCst)
                        && !stop_all.load(atomic::Ordering::SeqCst)
                    {
                        if let Err(e) = f() {
                            stop_all.store(true, atomic::Ordering::SeqCst);
                            return Err(e);
                        }
                    }
                    Ok(())
                }
            }),
        )
    }

    fn spawn_event_loop<F>(
        &self,
        service_id: ServiceIdRef,
        event_reader: event_log::SharedReader<P>,
        mut f: F,
    ) -> JoinHandle
    where
        F: for <'a> FnMut(&mut <<P as persistence::Persistence>::Connection as persistence::Connection>::Transaction<'a>, event_log::EventDetails) -> Result<()> + Send + Sync + 'static,
        P: persistence::Persistence + 'static,
    {
        let service_id = service_id.to_owned();

        let mut progress = {
            match (|| {
                let mut connection = self.persistence.get_connection()?;
                Ok(
                    if let Some(offset) = self.progress_store.load(&mut connection, &service_id)? {
                        offset
                    } else {
                        event_reader.get_start_offset()?
                    },
                )
            })() {
                // To avoid returning a `Result` directly from here, spawn a thread that will immediately terminate with an error,
                // just like the initial progress load was done from the spawned thread itself.
                Err(e) => {
                    return JoinHandle::new(
                        Arc::new(AtomicBool::new(false)),
                        thread::spawn(move || Err(e)),
                    )
                }
                Ok(o) => o,
            }
        };

        self.spawn_loop_raw({
            let progress_store = self.progress_store.clone();
            let persistence = self.persistence.clone();
            move || {
                let mut connection = persistence.get_connection()?;
                let mut transaction = connection.start_transaction()?;

                let (new_offset, mut events) = event_reader.read_tr(
                    &mut transaction,
                    progress.clone(),
                    1,
                    Some(std::time::Duration::from_secs(1)),
                )?;
                for event in events.drain(..) {
                    f(&mut transaction, event.details)?;

                    progress = new_offset;
                    progress_store.store_tr(&mut transaction, &service_id, new_offset)?;
                }
                transaction.commit()?;
                Ok(())
            }
        })
    }
}

/// Simple thread join wrapper that joins the thread on drop
///
/// TODO: Would it be better to have it set the `stop` flag toc terminate all threads
/// on drop?
pub struct JoinHandle {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<Result<()>>>,
}

impl JoinHandle {
    fn new(stop: Arc<AtomicBool>, handle: thread::JoinHandle<Result<()>>) -> Self {
        JoinHandle {
            stop,
            thread: Some(handle),
        }
    }
}

impl JoinHandle {
    fn join_mut(&mut self) -> Result<()> {
        if let Some(h) = self.thread.take() {
            h.join().map_err(|e| format_err!("join failed: {:?}", e))?
        } else {
            Ok(())
        }
    }

    #[allow(unused)]
    pub fn join(mut self) -> Result<()> {
        self.join_mut()
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.join_mut().expect("not failed")
    }
}
