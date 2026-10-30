// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Bounded completion support for blocking integration-test workers.

use std::panic::AssertUnwindSafe;
use std::panic::resume_unwind;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(10);

/// A blocking worker result that bounds how long a test waits for completion.
pub(crate) struct BlockingTestResult<T> {
    stage: &'static str,
    receiver: Receiver<thread::Result<T>>,
    worker: JoinHandle<()>,
}

impl<T> BlockingTestResult<T> {
    /// Waits for the worker, propagating its panic or reporting the stage
    /// timeout.
    pub(crate) fn finish(self) -> T {
        self.finish_with_timeout(COMPLETION_TIMEOUT)
    }

    /// Waits for the worker using the supplied timeout budget.
    fn finish_with_timeout(self, timeout: Duration) -> T {
        let Self {
            stage,
            receiver,
            worker,
        } = self;
        let result = receiver
            .recv_timeout(timeout)
            .unwrap_or_else(|error| panic!("blocking test worker timed out during {stage}: {error}"));
        worker.join().expect("completed worker thread should join");
        result.unwrap_or_else(|panic| resume_unwind(panic))
    }
}

/// Starts a blocking test operation whose completion is bounded by
/// [`BlockingTestResult::finish`].
pub(crate) fn spawn_with_timeout<T, F>(stage: &'static str, operation: F) -> BlockingTestResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = std::panic::catch_unwind(AssertUnwindSafe(operation));
        let _ = sender.send(result);
    });
    BlockingTestResult {
        stage,
        receiver,
        worker,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::spawn_with_timeout;

    #[test]
    fn test_blocking_worker_returns_value() {
        assert_eq!(spawn_with_timeout("normal completion", || 42).finish(), 42);
    }

    #[test]
    fn test_blocking_worker_propagates_panic() {
        let result =
            std::panic::catch_unwind(|| spawn_with_timeout("panic propagation", || panic!("worker panic")).finish());
        let panic = result.expect_err("worker panic should propagate");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic payload should be a string");
        assert_eq!(message, "worker panic");
    }

    #[test]
    fn test_blocking_worker_timeout_releases_controlled_worker() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let worker = spawn_with_timeout("controlled delay", move || {
            started_tx.send(()).expect("start receiver should be alive");
            let _ = release_rx.recv();
            let _ = completed_tx.send(());
        });

        let started = started_rx.recv_timeout(super::COMPLETION_TIMEOUT);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker.finish_with_timeout(Duration::from_millis(20))
        }));
        let released = release_tx.send(());
        let completed = completed_rx.recv_timeout(super::COMPLETION_TIMEOUT);

        assert!(started.is_ok(), "worker should start before timeout verification");
        assert!(released.is_ok(), "controlled worker should still be waiting");
        assert!(completed.is_ok(), "controlled worker should complete after release");
        let panic = result.expect_err("unfinished worker should time out");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic payload should be a string");
        assert!(message.contains("controlled delay"));
        assert!(message.contains("timed out"));
    }
}
