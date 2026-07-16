// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal state shared by Snowflake-family generators.

mod block_until_generated;
mod clock_defaults;
mod clock_observation;
mod generation_state;
mod restart_fence;
mod time_slice;

pub(crate) use block_until_generated::block_until_generated;
pub(crate) use clock_defaults::{
    default_blocking_sleeper,
    default_wall_clock,
};
pub(crate) use clock_observation::ClockObservation;
pub(crate) use generation_state::GenerationState;
pub(crate) use restart_fence::RestartFence;
pub(crate) use time_slice::TimeSlice;
