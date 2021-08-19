pub mod auction;

use anyhow::format_err;
use anyhow::Result;
use std::sync::{
    atomic::{self, AtomicBool},
    Arc,
};
use std::thread;

pub struct ServiceControl {
    stop: Arc<AtomicBool>,
}

impl ServiceControl {
    pub fn spawn_loop<F>(&self, f: F) -> JoinHandle
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
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
}

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
