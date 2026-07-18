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

use super::QubitSnowflakeLayout;
use super::internal::{
    BlockingSnowflake,
    SnowflakeCore,
};
use super::qubit_snowflake_generator_builder::QubitSnowflakeGeneratorBuilder;
use crate::{
    IdError,
    IdGenerator,
};

/// Qubit Snowflake generator.
///
/// This generator uses the Qubit fixed-header layout, including mode and
/// precision bits. The default constructor uses sequential mode, second
/// precision, the caller-provided host, and epoch `2018-12-02T00:00:00Z`.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::generate`] calls on
/// one shared live instance never return the same ID. A process should share
/// one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive host identifier when its layout and epoch can produce IDs in
/// the same namespace.
///
/// The default [`crate::RestartPolicy::Immediate`] allocates sequence zero in
/// the currently observed time slice without waiting. Allocation state is not
/// persisted. State loss or replacement can repeat an ID only when the
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
/// [`IdGenerator::generate`] may wait indefinitely when the wall clock stalls
/// or the injected timer does not cause wall time to progress. A backwards
/// clock movement within `max_clock_skew` is retried after waiting; a larger
/// movement returns [`IdError::ClockMovedBackwards`].
#[must_use]
pub struct QubitSnowflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<QubitSnowflakeLayout>,
}

impl QubitSnowflakeGenerator {
    /// Creates a generator with Qubit defaults.
    ///
    /// # Arguments
    ///
    /// * `host` - Host identifier in `0..=511`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the host
    /// field.
    ///
    /// # Panics
    ///
    /// Panics when the current wall time is equal to or later than the
    /// layout's exclusive expiration boundary.
    #[inline(always)]
    pub fn new(host: u64) -> Result<Self, IdError> {
        Self::builder(host).build()
    }

    /// Creates a configurable generator builder for the specified host.
    ///
    /// Host validation is performed when
    /// [`QubitSnowflakeGeneratorBuilder::build`] is called.
    ///
    /// # Arguments
    ///
    /// * `host` - Host identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable Qubit snowflake generator builder.
    #[inline(always)]
    pub fn builder(host: u64) -> QubitSnowflakeGeneratorBuilder {
        QubitSnowflakeGeneratorBuilder::new(host)
    }

    /// Constructs a generator from a validated builder configuration.
    ///
    /// # Arguments
    ///
    /// * `core` - Validated Qubit allocation core.
    /// * `timer` - Timer adapted for blocking generation.
    ///
    /// # Returns
    ///
    /// A generator containing the complete builder configuration.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<QubitSnowflakeLayout>,
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
    #[inline(always)]
    pub const fn layout(&self) -> &QubitSnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// Timestamp origin.
    #[must_use]
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
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
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }

    /// Returns the maximum tolerated backwards clock movement.
    ///
    /// # Returns
    ///
    /// Maximum tolerated raw wall-clock rollback.
    #[must_use]
    #[inline(always)]
    pub const fn max_clock_skew(&self) -> Duration {
        self.inner.core().max_clock_skew()
    }

    /// Generates an ID for an explicit time and sequence.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Arguments
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
    /// Returns [`IdError::TimeBeforeEpoch`] if `time` is before the configured
    /// epoch. Returns [`IdError::TimestampOverflow`] or
    /// [`IdError::SequenceOverflow`] when the computed timestamp or provided
    /// sequence does not fit the layout.
    #[inline(always)]
    pub fn generate_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        self.inner.core().generate_at(time, sequence)
    }
}

impl IdGenerator<u64> for QubitSnowflakeGenerator {
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
    /// Returns an allocation error or [`IdError::WaitFailed`] when a retry
    /// delay cannot be completed.
    #[inline(always)]
    fn generate(&self) -> Result<u64, IdError> {
        self.inner.generate()
    }
}
