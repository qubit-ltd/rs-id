// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared deterministic time support for integration tests.

#[cfg(feature = "qubit-snowflake")]
mod closure_wall_clock;
#[cfg(feature = "qubit-snowflake")]
mod failing_blocking_sleeper;
mod manual_time;
mod panicking_wall_clock;

#[cfg(feature = "qubit-snowflake")]
pub(crate) use closure_wall_clock::ClosureWallClock;
#[cfg(feature = "qubit-snowflake")]
pub(crate) use failing_blocking_sleeper::FailingBlockingSleeper;
pub(crate) use manual_time::ManualTime;
pub(crate) use panicking_wall_clock::PanickingWallClock;
