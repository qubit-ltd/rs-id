// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use qubit_clock::Timer;

use super::super::internal::{
    BlockingSnowflake,
    SnowflakeCore,
};
use super::SnowflakeLayout;
use super::snowflake_generator_builder::SnowflakeGeneratorBuilder;
use crate::{
    AsyncIdGenerator,
    GenerationAttempt,
    Id,
    IdGenerationError,
    IdGenerationFuture,
    IdGenerator,
    TryIdGenerator,
};

/// Qubit Snowflake generator.
///
/// This generator uses the Qubit fixed-header layout, including mode and
/// precision bits. The default constructor uses sequential mode, second
/// precision, the caller-provided host, and epoch `2018-12-02T00:00:00Z`.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::generate`]
/// calls on one shared live instance never return the same ID. A process should
/// share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive host identifier when its layout and epoch can produce IDs in
/// the same namespace.
///
/// The default [`crate::RestartPolicy::Immediate`] starts allocation in the
/// first observed time slice. [`crate::RestartPolicy::WaitNextSlice`] is an
/// explicit opt-in that skips that slice. Allocation state is not persisted.
/// State loss or replacement can repeat an ID only when the
/// instances use the same effective identity (`host`), layout (`mode` and
/// `precision`), and reference time (`epoch`), allocate in the same logical
/// time slice, and use overlapping sequence ranges.
///
/// [`crate::RestartPolicy::WaitNextSlice`] waits until after the first observed
/// time slice. It reduces sequential-replacement risk only when that slice is
/// not earlier than the predecessor's last allocated slice. Because predecessor
/// state is not persisted, clock rollback across a restart can still repeat
/// IDs. The policy also does not coordinate concurrent same-identity instances,
/// which can cross the fence together and allocate overlapping sequence ranges.
/// Such deployments require external exclusivity.
///
/// # Blocking and clock behavior
///
/// [`IdGenerator::generate`] may wait indefinitely when the wall clock
/// stalls or the injected timer does not cause wall time to progress. A
/// backwards clock movement within `max_clock_skew` is retried after waiting; a
/// larger movement returns [`IdGenerationError::ClockMovedBackwards`].
#[derive(Clone)]
#[must_use]
pub struct SnowflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<SnowflakeLayout>,
}

impl SnowflakeGenerator {
    /// Creates a generator with Qubit defaults.
    ///
    /// # Parameters
    ///
    /// * `host` - Host identifier in `0..=511`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::HostOutOfRange`] when `host` does not fit
    /// in the host field, [`IdGenerationError::ExpirationTimeOverflow`]
    /// when the lifetime boundary cannot be represented,
    /// [`IdGenerationError::EpochAhead`] when the default epoch is later than
    /// the current wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the current wall time
    /// has reached that boundary.
    #[inline(always)]
    pub fn new(host: u64) -> Result<Self, IdGenerationError> {
        Self::builder(host).build()
    }

    /// Creates a configurable generator builder for the specified host.
    ///
    /// Host validation is performed when
    /// [`SnowflakeGeneratorBuilder::build`] is called.
    ///
    /// # Parameters
    ///
    /// * `host` - Host identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable Qubit snowflake generator builder.
    #[inline(always)]
    pub fn builder(host: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(host)
    }

    /// Constructs a generator from a validated builder configuration.
    ///
    /// # Parameters
    ///
    /// * `core` - Validated Qubit allocation core.
    /// * `timer` - Timer adapted for blocking generation.
    ///
    /// # Returns
    ///
    /// A generator containing the complete builder configuration.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<SnowflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: BlockingSnowflake::new(core, timer),
        }
    }

    /// Returns the Qubit bit layout.
    ///
    /// # Returns
    ///
    /// Layout used to compose generated IDs.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::SnowflakeGenerator;
    /// let generator = SnowflakeGenerator::new(7).expect("valid host");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub fn layout(&self) -> &SnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// Timestamp origin.
    #[must_use]
    #[inline(always)]
    pub fn epoch(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive timestamp expiration boundary.
    ///
    /// The generator is expired when the wall clock is equal to or later than
    /// this time.
    ///
    /// # Returns
    ///
    /// Exclusive expiration boundary.
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

    /// Generates the next Qubit Snowflake ID.
    ///
    /// This inherent method is convenient for concrete callers. Use
    /// [`IdGenerator`] when an object-safe dynamic-dispatch boundary is
    /// needed.
    ///
    /// # Returns
    ///
    /// The next generated Qubit Snowflake ID.
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
    pub fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Id>, IdGenerationError> {
        self.inner
            .try_generate()
            .map(|attempt| attempt.map(Id::from))
    }

    /// Generates the next Qubit Snowflake ID asynchronously.
    ///
    /// The allocation lock is released before every timer registration and
    /// await. Dropping the returned future cancels an incomplete timer wait.
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
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `time` - Time to encode.
    /// * `sequence` - Sequence value inside the encoded time slice.
    ///
    /// # Returns
    ///
    /// Encoded ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] if `time` is before the
    /// configured epoch, [`IdGenerationError::GeneratorExpired`] if `time`
    /// has reached the exclusive expiration boundary, or
    /// [`IdGenerationError::SequenceOverflow`] when `sequence` does not fit
    /// the layout.
    #[inline(always)]
    pub fn compose_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<Id, IdGenerationError> {
        self.inner.core().compose_at(time, sequence).map(Id::from)
    }
}

impl IdGenerator for SnowflakeGenerator {
    /// Generates the next Qubit snowflake ID.
    ///
    /// Timestamp and sequence pairs are reserved while holding the generator
    /// mutex. When the current sequence range is exhausted, this method
    /// releases the mutex, waits for a later time slice, and then competes
    /// for a new reservation. The method can therefore block for
    /// approximately one time slice while the clock advances normally, or
    /// longer while tolerating a configured backwards clock skew.
    ///
    /// # Returns
    ///
    /// The next generated Qubit snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] when the wall clock
    /// precedes the epoch, [`IdGenerationError::GeneratorExpired`] at the
    /// lifetime boundary, [`IdGenerationError::ClockMovedBackwards`] when
    /// rollback exceeds the configured tolerance, or
    /// [`IdGenerationError::WaitFailed`] when a retry wait cannot be
    /// registered or completed.
    #[inline(always)]
    fn generate(&self) -> Result<Id, IdGenerationError> {
        SnowflakeGenerator::generate(self)
    }
}

impl TryIdGenerator for SnowflakeGenerator {
    /// Attempts one non-blocking Qubit Snowflake allocation.
    #[inline]
    fn try_generate(&self) -> Result<GenerationAttempt<Id>, IdGenerationError> {
        SnowflakeGenerator::try_generate(self)
    }
}

impl AsyncIdGenerator for SnowflakeGenerator {
    /// Generates a Qubit Snowflake ID asynchronously.
    #[inline]
    fn generate_async(&self) -> IdGenerationFuture<'_, Id, IdGenerationError> {
        Box::pin(SnowflakeGenerator::generate_async(self))
    }
}
