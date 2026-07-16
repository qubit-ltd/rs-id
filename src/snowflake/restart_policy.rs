// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Restart policy for Snowflake-family generators.

/// Determines when a fresh Snowflake generator may allocate its first ID.
///
/// Allocation state is not persisted. The default [`Self::Immediate`] policy
/// minimizes startup latency but can repeat IDs after state loss when the old
/// and replacement instances use the same effective identity, layout, and
/// reference time, allocate in the same logical time slice, and use
/// overlapping sequence ranges.
///
/// [`Self::WaitNextSlice`] reduces that risk for sequential replacement only.
/// It does not coordinate concurrently running instances with the same
/// identity; external exclusivity is still required.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestartPolicy {
    /// Allocates immediately and may repeat IDs after same-slice state loss.
    ///
    /// This is the default because it preserves immediate startup and maximum
    /// usable time-slice capacity.
    #[default]
    Immediate,
    /// Waits until the slice after the first observed slice.
    ///
    /// This protects a new instance that starts after its predecessor has
    /// stopped, but it does not coordinate concurrent same-identity instances.
    WaitNextSlice,
}
