// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Checked exclusive-expiration calculations for Snowflake layouts.

use std::time::{
    Duration,
    SystemTime,
};

use crate::IdError;

/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Calculates the exclusive expiration time for a timestamp layout.
///
/// The timestamp range `0..=max_timestamp` contains `max_timestamp + 1`
/// complete units. The returned boundary is therefore the first instant that
/// cannot be encoded by the layout.
///
/// # Arguments
///
/// * `origin` - Timestamp origin represented by encoded timestamp zero.
/// * `time_unit` - Duration represented by one encoded timestamp unit.
/// * `max_timestamp` - Maximum timestamp supported by the layout.
///
/// # Returns
///
/// The exclusive expiration boundary.
///
/// # Errors
///
/// Returns [`IdError::ExpirationTimeOverflow`] when the lifetime duration or
/// resulting [`SystemTime`] cannot be represented.
pub(crate) fn expiration_time(
    origin: SystemTime,
    time_unit: Duration,
    max_timestamp: u64,
) -> Result<SystemTime, IdError> {
    let overflow = || IdError::ExpirationTimeOverflow {
        origin,
        time_unit,
        max_timestamp,
    };
    let unit_count = max_timestamp.checked_add(1).ok_or_else(&overflow)?;
    let total_nanos = time_unit
        .as_nanos()
        .checked_mul(u128::from(unit_count))
        .ok_or_else(&overflow)?;
    let seconds = u64::try_from(total_nanos / NANOS_PER_SECOND)
        .map_err(|_| overflow())?;
    let subsec_nanos = (total_nanos % NANOS_PER_SECOND) as u32;
    let lifetime = Duration::new(seconds, subsec_nanos);
    origin.checked_add(lifetime).ok_or_else(overflow)
}

/// Panics when a generator configuration has reached its expiration.
///
/// # Arguments
///
/// * `algorithm` - Algorithm name included in the panic message.
/// * `now` - Current wall time observed during construction.
/// * `expires_at` - Exclusive expiration boundary.
///
/// # Panics
///
/// Panics when `now` is equal to or later than `expires_at`.
#[inline]
pub(crate) fn panic_if_expired(
    algorithm: &'static str,
    now: SystemTime,
    expires_at: SystemTime,
) {
    assert!(
        now < expires_at,
        "{algorithm} generator expired: current time {now:?} reached exclusive expiration {expires_at:?}",
    );
}
