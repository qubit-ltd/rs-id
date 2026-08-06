// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Checked exclusive-expiration calculations for Snowflake layouts.

use std::time::{
    Duration,
    SystemTime,
};

use crate::IdGenerationError;

/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Calculates the exclusive expiration time for a timestamp layout.
///
/// The timestamp range `0..=max_timestamp` contains `max_timestamp + 1`
/// complete units. The returned boundary is therefore the first instant that
/// cannot be encoded by the layout.
///
/// # Parameters
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
/// Returns [`IdGenerationError::ExpirationTimeOverflow`] when the lifetime
/// duration or resulting [`SystemTime`] cannot be represented.
pub(crate) fn expiration_time(
    origin: SystemTime,
    time_unit: Duration,
    max_timestamp: u64,
) -> Result<SystemTime, IdGenerationError> {
    let unit_count = u128::from(max_timestamp) + 1;
    let Some(total_nanos) = time_unit.as_nanos().checked_mul(unit_count) else {
        return Err(IdGenerationError::ExpirationTimeOverflow {
            origin,
            time_unit,
            max_timestamp,
        });
    };
    let Ok(seconds) = u64::try_from(total_nanos / NANOS_PER_SECOND) else {
        return Err(IdGenerationError::ExpirationTimeOverflow {
            origin,
            time_unit,
            max_timestamp,
        });
    };
    let subsec_nanos = (total_nanos % NANOS_PER_SECOND) as u32;
    let lifetime = Duration::new(seconds, subsec_nanos);
    origin.checked_add(lifetime).ok_or(
        IdGenerationError::ExpirationTimeOverflow {
            origin,
            time_unit,
            max_timestamp,
        },
    )
}

/// Validates the configured Snowflake lifetime against an observed wall time.
///
/// # Parameters
///
/// * `epoch` - Timestamp origin represented by encoded timestamp zero.
/// * `expires_at` - Exclusive expiration boundary for the configured layout.
/// * `current_time` - Wall time observed while validating the builder.
///
/// # Returns
///
/// Returns `Ok(())` when the epoch has begun and the generator has not expired.
///
/// # Errors
///
/// Returns [`IdGenerationError::EpochAhead`] when `epoch` is later than
/// `current_time`, or [`IdGenerationError::GeneratorExpired`] when
/// `current_time` has reached `expires_at`.
pub(crate) fn validate_generator_lifetime(
    epoch: SystemTime,
    expires_at: SystemTime,
    current_time: SystemTime,
) -> Result<(), IdGenerationError> {
    validate_generator_epoch(epoch, current_time)?;
    if current_time >= expires_at {
        return Err(IdGenerationError::GeneratorExpired {
            observed_at: current_time,
            expires_at,
        });
    }
    Ok(())
}

/// Rejects a timestamp epoch that has not started at the observed wall time.
///
/// # Parameters
///
/// * `epoch` - Timestamp origin represented by encoded timestamp zero.
/// * `current_time` - Wall time observed while validating the builder.
///
/// # Returns
///
/// Returns `Ok(())` when `epoch` is at or before `current_time`.
///
/// # Errors
///
/// Returns [`IdGenerationError::EpochAhead`] when `epoch` is later than
/// `current_time`.
pub(crate) fn validate_generator_epoch(
    epoch: SystemTime,
    current_time: SystemTime,
) -> Result<(), IdGenerationError> {
    if epoch > current_time {
        return Err(IdGenerationError::EpochAhead {
            epoch,
            current_time,
        });
    }
    Ok(())
}
