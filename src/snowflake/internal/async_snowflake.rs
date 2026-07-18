// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Asynchronous driver for the shared Snowflake allocation core.

use std::sync::Arc;

use qubit_clock::Timer;

use super::{
    GenerationAttempt,
    SnowflakeCore,
    SnowflakeLayoutSpec,
};
use crate::IdError;

/// Adapts a non-waiting Snowflake core to asynchronous generation.
pub(crate) struct AsyncSnowflake<L> {
    /// Shared allocation and layout logic.
    core: SnowflakeCore<L>,
    /// Timer used to register asynchronous retry waits.
    timer: Arc<dyn Timer>,
}

impl<L> AsyncSnowflake<L>
where
    L: SnowflakeLayoutSpec,
{
    /// Creates an asynchronous driver for a validated core and timer.
    ///
    /// # Parameters
    ///
    /// * `core` - Shared non-waiting allocation core.
    /// * `timer` - Timer used for asynchronous retry waits.
    ///
    /// # Returns
    ///
    /// An asynchronous Snowflake driver.
    #[inline]
    pub(crate) const fn new(
        core: SnowflakeCore<L>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self { core, timer }
    }

    /// Generates an ID, yielding across retryable allocation outcomes.
    ///
    /// The allocation lock is released before every timer registration and
    /// await. Dropping the returned future cancels an incomplete timer wait.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns an allocation error or [`IdError::WaitFailed`] when the timer
    /// cannot register a retry wait.
    pub(crate) async fn generate(&self) -> Result<u64, IdError> {
        loop {
            match self.core.try_generate()? {
                GenerationAttempt::Generated(id) => return Ok(id),
                GenerationAttempt::RetryAfter(duration) => {
                    self.timer
                        .after(duration)
                        .map_err(|source| IdError::WaitFailed { source })?
                        .await;
                }
            }
        }
    }

    /// Returns the shared non-waiting allocation core.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn core(&self) -> &SnowflakeCore<L> {
        &self.core
    }
}
