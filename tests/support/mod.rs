// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared deterministic time support for integration tests.

mod async_test_timeout;
mod async_test_timeout_tests;
mod blocking_test_timeout;
#[cfg(feature = "qubit-snowflake")]
mod completion_failing_timer;
mod manual_time;
mod time_boundaries;

pub(crate) use async_test_timeout::complete_with_timeout;
pub(crate) use blocking_test_timeout::spawn_with_timeout;
#[cfg(feature = "qubit-snowflake")]
pub(crate) use completion_failing_timer::CompletionFailingTimer;
pub(crate) use manual_time::ManualTime;
pub(crate) use time_boundaries::latest_representable_whole_second;
