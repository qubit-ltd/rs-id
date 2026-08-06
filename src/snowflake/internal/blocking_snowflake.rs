// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
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
use crate::IdGenerationError;

/// Adapts a non-waiting Snowflake core to synchronous generation.
#[derive(Clone)]
pub(crate) struct BlockingSnowflake<L> {
    /// Shared allocation and layout logic.
    core: Arc<SnowflakeCore<L>>,
    /// Blocking adapter over the injected timer.
    sleeper: BlockingSleeper,
}

impl<L> BlockingSnowflake<L>
where
    L: SnowflakeLayoutSpec,
{
    /// Creates a blocking driver for a validated core and timer.
    ///
    /// # Parameters
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
            core: Arc::new(core),
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
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] when the clock precedes
    /// the epoch, [`IdGenerationError::GeneratorExpired`] at the lifetime
    /// boundary, [`IdGenerationError::ClockMovedBackwards`] when rollback
    /// exceeds the configured tolerance, or
    /// [`IdGenerationError::WaitFailed`] when the timer cannot register or
    /// complete a retry wait.
    pub(crate) fn generate(&self) -> Result<u64, IdGenerationError> {
        loop {
            match self.try_generate()? {
                GenerationAttempt::Generated(id) => return Ok(id),
                GenerationAttempt::RetryAfter { delay } => {
                    self.sleeper.sleep_for(delay).map_err(|source| {
                        IdGenerationError::WaitFailed { source }
                    })?;
                }
            }
        }
    }

    /// Performs one non-blocking allocation attempt.
    pub(crate) fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<u64>, IdGenerationError> {
        self.core.try_generate()
    }

    /// Generates an ID asynchronously, yielding across retryable outcomes.
    pub(crate) async fn generate_async(
        &self,
    ) -> Result<u64, IdGenerationError> {
        loop {
            match self.try_generate()? {
                GenerationAttempt::Generated(id) => return Ok(id),
                GenerationAttempt::RetryAfter { delay } => {
                    self.sleeper
                        .timer()
                        .after(delay)
                        .map_err(|source| IdGenerationError::WaitFailed {
                            source,
                        })?
                        .await
                        .map_err(|source| IdGenerationError::WaitFailed {
                            source,
                        })?;
                }
            }
        }
    }

    /// Returns the shared non-waiting allocation core.
    ///
    /// # Returns
    ///
    /// The allocation core adapted by this blocking driver.
    #[must_use]
    #[inline(always)]
    pub(crate) fn core(&self) -> &SnowflakeCore<L> {
        self.core.as_ref()
    }
}
