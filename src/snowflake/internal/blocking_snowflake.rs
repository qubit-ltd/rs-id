// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Blocking driver for the shared Snowflake allocation core.

use std::sync::Arc;

use qubit_clock::{
    BlockingSleeper,
    Timer,
};

use super::{
    GenerationAttempt,
    SnowflakeCore,
    SnowflakeLayoutSpec,
};
use crate::IdError;

/// Adapts a non-waiting Snowflake core to synchronous generation.
pub(crate) struct BlockingSnowflake<L> {
    /// Shared allocation and layout logic.
    core: SnowflakeCore<L>,
    /// Blocking adapter over the injected timer.
    sleeper: BlockingSleeper,
}

impl<L> BlockingSnowflake<L>
where
    L: SnowflakeLayoutSpec,
{
    /// Creates a blocking driver for a validated core and timer.
    ///
    /// # Arguments
    ///
    /// * `core` - Shared non-waiting allocation core.
    /// * `timer` - Timer adapted to blocking retry waits.
    ///
    /// # Returns
    ///
    /// A synchronous Snowflake driver.
    #[inline]
    pub(crate) fn new(core: SnowflakeCore<L>, timer: Arc<dyn Timer>) -> Self {
        Self {
            core,
            sleeper: BlockingSleeper::new(timer),
        }
    }

    /// Generates an ID, blocking across retryable allocation outcomes.
    ///
    /// The allocation lock is released before every blocking wait.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns an allocation error or [`IdError::WaitFailed`] when the timer
    /// cannot complete a retry wait.
    pub(crate) fn generate(&self) -> Result<u64, IdError> {
        loop {
            match self.core.try_generate()? {
                GenerationAttempt::Generated(id) => return Ok(id),
                GenerationAttempt::RetryAfter(duration) => {
                    self.sleeper
                        .sleep_for(duration)
                        .map_err(|source| IdError::WaitFailed { source })?;
                }
            }
        }
    }

    /// Returns the shared non-waiting allocation core.
    #[inline(always)]
    pub(crate) const fn core(&self) -> &SnowflakeCore<L> {
        &self.core
    }
}
