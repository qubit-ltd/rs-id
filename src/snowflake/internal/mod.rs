// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal state shared by Snowflake-family generators.

mod async_snowflake;
mod blocking_snowflake;
mod clock_defaults;
mod clock_observation;
#[cfg(any(feature = "qubit-snowflake", feature = "classic-snowflake"))]
mod default_epoch;
mod expiration_time;
mod generation_attempt;
mod generation_state;
mod restart_fence;
mod snowflake_core;
mod snowflake_layout_spec;
mod time_slice;

pub(crate) use async_snowflake::AsyncSnowflake;
pub(crate) use blocking_snowflake::BlockingSnowflake;
pub(crate) use clock_defaults::{
    default_timer,
    default_wall_clock,
};
pub(crate) use clock_observation::ClockObservation;
#[cfg(any(feature = "qubit-snowflake", feature = "classic-snowflake"))]
pub(crate) use default_epoch::DEFAULT_SNOWFLAKE_EPOCH_MILLIS;
pub(crate) use expiration_time::{
    expiration_time,
    panic_if_expired,
};
pub(crate) use generation_attempt::GenerationAttempt;
pub(crate) use generation_state::GenerationState;
pub(crate) use restart_fence::RestartFence;
pub(crate) use snowflake_core::SnowflakeCore;
pub(crate) use snowflake_layout_spec::SnowflakeLayoutSpec;
pub(crate) use time_slice::TimeSlice;
