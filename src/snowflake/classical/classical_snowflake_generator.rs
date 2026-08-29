// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous classic 41/10/12 Snowflake generator.

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::super::internal::BlockingSnowflake;
use super::super::internal::SnowflakeCore;
use super::ClassicalSnowflakeGeneratorBuilder;
use super::ClassicalSnowflakeLayout;
use crate::AsyncIdGenerator;
use crate::GenerationAttempt;
use crate::Id;
use crate::IdGenerationError;
use crate::IdGenerationFuture;
use crate::IdGenerator;
use crate::TryIdGenerator;

/// Generates classic Snowflake IDs with 41 timestamp, 10 node, and 12
/// sequence bits.
///
/// The generator is thread-safe. One shared live instance never returns the
/// same ID twice, provided its node identifier is exclusive within the ID
/// namespace. [`IdGenerator::generate`] blocks when sequence capacity
/// is exhausted until the injected timer allows wall time to advance. A
/// backwards clock movement within [`Self::max_clock_skew`] is retried after
/// waiting; a larger movement returns
/// [`IdGenerationError::ClockMovedBackwards`].
#[derive(Clone)]
#[must_use]
pub struct ClassicalSnowflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<ClassicalSnowflakeLayout>,
}

impl ClassicalSnowflakeGenerator {
    /// Creates a generator with the default Qubit epoch.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier in `0..=1023`.
    ///
    /// # Returns
    ///
    /// A configured classic Snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::NodeOutOfRange`] when `node_id` does not
    /// fit the 10-bit node field,
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the lifetime
    /// boundary cannot be represented,
    /// [`IdGenerationError::EpochAhead`] when the default epoch is later than
    /// the current wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the current wall time
    /// has reached that boundary.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdGenerationError> {
        Self::builder(node_id).build()
    }

    /// Creates a configurable builder for a node identifier.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A classic Snowflake generator builder.
    #[inline(always)]
    pub fn builder(node_id: u64) -> ClassicalSnowflakeGeneratorBuilder {
        ClassicalSnowflakeGeneratorBuilder::new(node_id)
    }

    /// Creates a public generator from a validated core and timer.
    ///
    /// # Parameters
    ///
    /// * `core` - Validated allocation core.
    /// * `timer` - Timer adapted to blocking retry waits.
    ///
    /// # Returns
    ///
    /// A generator backed by `core` and `timer`.
    #[inline]
    pub(super) fn from_core(core: SnowflakeCore<ClassicalSnowflakeLayout>, timer: Arc<dyn Timer>) -> Self {
        Self {
            inner: BlockingSnowflake::new(core, timer),
        }
    }

    /// Returns the configured classic Snowflake layout.
    ///
    /// # Returns
    ///
    /// The layout used to compose generated IDs.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::ClassicalSnowflakeGenerator;
    /// let generator = ClassicalSnowflakeGenerator::new(7).expect("valid node");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub fn layout(&self) -> &ClassicalSnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured timestamp origin.
    ///
    /// # Returns
    ///
    /// The timestamp origin represented by timestamp zero.
    #[must_use]
    #[inline(always)]
    pub fn epoch(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    ///
    /// # Returns
    ///
    /// The first wall time that cannot be represented by this generator.
    #[must_use]
    #[inline(always)]
    pub fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }

    /// Returns the maximum tolerated backwards clock movement.
    ///
    /// # Returns
    ///
    /// Maximum tolerated raw wall-clock rollback.
    #[must_use]
    #[inline(always)]
    pub fn max_clock_skew(&self) -> Duration {
        self.inner.core().max_clock_skew()
    }

    /// Generates the next classic Snowflake ID.
    ///
    /// This inherent method is convenient for concrete callers. Use
    /// [`IdGenerator`] when an object-safe dynamic-dispatch boundary is
    /// needed.
    ///
    /// # Returns
    ///
    /// The next generated classic Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as the [`IdGenerator::generate`]
    /// implementation.
    #[inline(always)]
    pub fn generate(&self) -> Result<Id, IdGenerationError> {
        self.inner.generate().map(Id::from)
    }

    /// Attempts to generate an ID without sleeping or awaiting.
    ///
    /// # Returns
    ///
    /// A generated ID or the minimum delay before another attempt can make
    /// progress.
    ///
    /// # Errors
    ///
    /// Returns the non-retryable allocation errors described by
    /// [`IdGenerator::generate`].
    #[inline]
    pub fn try_generate(&self) -> Result<GenerationAttempt<Id>, IdGenerationError> {
        self.inner.try_generate().map(|attempt| attempt.map(Id::from))
    }

    /// Generates the next classic Snowflake ID asynchronously.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`IdGenerator::generate`].
    pub async fn generate_async(&self) -> Result<Id, IdGenerationError> {
        self.inner.generate_async().await.map(Id::from)
    }

    /// Composes an ID for an explicit time and sequence.
    ///
    /// This operation is stateless and provides no uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time to encode.
    /// * `sequence` - Sequence to encode within that millisecond.
    ///
    /// # Returns
    ///
    /// A classic Snowflake ID containing the specified timestamp and sequence.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] if `time` is before the
    /// configured epoch, [`IdGenerationError::GeneratorExpired`] if `time`
    /// has reached the exclusive expiration boundary, or
    /// [`IdGenerationError::SequenceOverflow`] when `sequence` does not fit
    /// the layout.
    #[inline(always)]
    pub fn compose_at(&self, time: SystemTime, sequence: u64) -> Result<Id, IdGenerationError> {
        self.inner.core().compose_at(time, sequence).map(Id::from)
    }
}

impl IdGenerator for ClassicalSnowflakeGenerator {
    /// Generates the next classic Snowflake ID.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] when the wall clock
    /// precedes the epoch, [`IdGenerationError::GeneratorExpired`] at the
    /// lifetime boundary, [`IdGenerationError::ClockMovedBackwards`] when a
    /// wall-clock rollback exceeds the configured tolerance, or
    /// [`IdGenerationError::WaitFailed`] when a retry wait cannot be
    /// registered or completed.
    #[inline(always)]
    fn generate(&self) -> Result<Id, IdGenerationError> {
        ClassicalSnowflakeGenerator::generate(self)
    }
}

impl TryIdGenerator for ClassicalSnowflakeGenerator {
    /// Attempts one non-blocking classic Snowflake allocation.
    #[inline]
    fn try_generate(&self) -> Result<GenerationAttempt<Id>, IdGenerationError> {
        ClassicalSnowflakeGenerator::try_generate(self)
    }
}

impl AsyncIdGenerator for ClassicalSnowflakeGenerator {
    /// Generates a classic Snowflake ID asynchronously.
    #[inline]
    fn generate_async(&self) -> IdGenerationFuture<'_, Id, IdGenerationError> {
        Box::pin(ClassicalSnowflakeGenerator::generate_async(self))
    }
}
