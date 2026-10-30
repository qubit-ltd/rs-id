// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Timeout protection for asynchronous test coordination.

use std::future::Future;
use std::time::Duration;

/// Completes a future within the test coordination timeout.
pub(crate) async fn complete_with_timeout<F>(stage: &str, future: F) -> F::Output
where
    F: Future,
{
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .unwrap_or_else(|_| panic!("test coordination timed out: {stage}"))
}
