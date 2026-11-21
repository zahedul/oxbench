//! Runs slow work, such as bcrypt or hashing a large file, on a background thread so the
//! window stays responsive.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use eframe::egui;

/// The latest background job of one kind and its outcome.
#[derive(Default)]
pub enum JobSlot<T> {
    #[default]
    Idle,
    Running(Receiver<T>),
    Finished(T),
    /// The worker stopped without sending a result.
    Lost,
}

impl<T: Send + 'static> JobSlot<T> {
    /// Starts `work`, replacing any job still running; the replaced job's result is dropped.
    pub fn start(&mut self, ctx: &egui::Context, work: impl FnOnce() -> T + Send + 'static) {
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("oxbench-job".to_owned())
            .spawn(move || {
                let result = work();
                // Sending fails only when a newer job replaced this one, so the result is stale.
                if sender.send(result).is_ok() {
                    ctx.request_repaint();
                }
            });
        *self = match spawned {
            Ok(_) => JobSlot::Running(receiver),
            Err(_) => JobSlot::Lost,
        };
    }

    /// Picks up a finished result. Call it once per frame before reading the slot.
    pub fn poll(&mut self) {
        let JobSlot::Running(receiver) = self else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => *self = JobSlot::Finished(result),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => *self = JobSlot::Lost,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, JobSlot::Running(_))
    }

    pub fn finished(&self) -> Option<&T> {
        match self {
            JobSlot::Finished(result) => Some(result),
            _ => None,
        }
    }
}

/// Steps the UI until `is_done` holds, for tests that wait on a background job.
#[cfg(test)]
pub fn step_until<S>(harness: &mut egui_kittest::Harness<'_, S>, is_done: impl Fn(&S) -> bool) {
    const ATTEMPTS: usize = 1000;
    for _ in 0..ATTEMPTS {
        harness.step();
        if is_done(harness.state()) {
            harness.run();
            return;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the background job did not finish within 10 seconds");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for<T: Send + 'static>(slot: &mut JobSlot<T>) {
        for _ in 0..500 {
            slot.poll();
            if !slot.is_running() {
                return;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn delivers_the_result() {
        let ctx = egui::Context::default();
        let mut slot = JobSlot::default();
        slot.start(&ctx, || 6 * 7);
        wait_for(&mut slot);
        assert_eq!(slot.finished(), Some(&42));
    }

    #[test]
    fn a_newer_job_replaces_the_old_one() {
        let ctx = egui::Context::default();
        let mut slot = JobSlot::default();
        slot.start(&ctx, || {
            thread::sleep(std::time::Duration::from_millis(50));
            "old"
        });
        slot.start(&ctx, || "new");
        wait_for(&mut slot);
        assert_eq!(slot.finished(), Some(&"new"));
    }

    #[test]
    fn reports_a_worker_that_stopped() {
        let ctx = egui::Context::default();
        let mut slot: JobSlot<u8> = JobSlot::default();
        slot.start(&ctx, || panic!("worker failed"));
        wait_for(&mut slot);
        assert!(matches!(slot, JobSlot::Lost));
    }
}
