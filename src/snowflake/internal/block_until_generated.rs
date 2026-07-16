// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Adapts non-sleeping generation attempts to blocking generation.

use qubit_clock::BlockingSleeper;

use crate::{
    GenerationOutcome,
    IdError,
};

/// Repeats allocation attempts using an injected blocking sleeper.
///
/// `T` is the generated ID representation and `F` is the stateful one-attempt
/// callback invoked until it produces an ID or error.
///
/// # Arguments
///
/// * `sleeper` - Sleeper used for retry delays.
/// * `attempt` - One generation attempt that does not invoke a sleeper.
///
/// # Returns
///
/// The first generated ID.
///
/// # Errors
///
/// Returns an error from `attempt`, or [`IdError::SleepFailed`] when the
/// sleeper cannot complete a retry delay.
pub(crate) fn block_until_generated<T, F>(
    sleeper: &dyn BlockingSleeper,
    mut attempt: F,
) -> Result<T, IdError>
where
    F: FnMut() -> Result<GenerationOutcome<T>, IdError>,
{
    loop {
        match attempt()? {
            GenerationOutcome::Generated(id) => return Ok(id),
            GenerationOutcome::RetryAfter(duration) => {
                sleeper
                    .sleep_for(duration)
                    .map_err(|source| IdError::SleepFailed { source })?;
            }
        }
    }
}
