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
        let Self {
            stage,
            receiver,
            worker,
        } = self;
        let result = receiver
            .recv_timeout(COMPLETION_TIMEOUT)
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
        let (release, wait_for_release) = mpsc::channel();
        let worker = spawn_with_timeout("controlled delay", move || {
            let _ = wait_for_release.recv();
        });
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| worker.finish()));
        let panic = result.expect_err("unfinished worker should time out");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic payload should be a string");
        assert!(message.contains("controlled delay"));
        release.send(()).expect("worker should still be waiting");
        std::thread::sleep(Duration::from_millis(20));
    }
}
