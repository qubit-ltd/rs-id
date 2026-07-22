// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared deterministic time support for integration tests.

#[cfg(feature = "qubit-snowflake")]
mod completion_failing_timer;
mod manual_time;

#[cfg(feature = "qubit-snowflake")]
pub(crate) use completion_failing_timer::CompletionFailingTimer;
pub(crate) use manual_time::ManualTime;
