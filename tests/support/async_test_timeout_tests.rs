// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for asynchronous test coordination timeouts.

use super::complete_with_timeout;

#[tokio::test]
async fn test_async_coordination_timeout_preserves_output() {
    assert_eq!(complete_with_timeout("ready result", async { 42 }).await, 42);
}

#[tokio::test(start_paused = true)]
#[should_panic(expected = "test coordination timed out: missing waiter")]
async fn test_async_coordination_timeout_reports_stage() {
    complete_with_timeout("missing waiter", std::future::pending::<()>()).await;
}
