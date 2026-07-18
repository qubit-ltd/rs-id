// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous Sonyflake-style 63-bit ID generator.

use std::sync::Arc;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::internal::{
    BlockingSnowflake,
    SnowflakeCore,
};
use super::{
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
};
use crate::{
    IdError,
    IdGenerator,
};

/// Default Sonyflake start time as Unix epoch milliseconds.
pub(super) const DEFAULT_START_MILLIS: u64 = 1_735_689_600_000;

/// Generates Sonyflake-style IDs with configurable time, sequence, and
/// machine fields.
///
/// The generator is thread-safe. One shared live instance never returns the
/// same ID twice, provided its machine identifier is exclusive within the ID
/// namespace. Retry waits use the timer injected through the builder.
#[must_use]
pub struct SonyflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<SonyflakeLayout>,
}

impl SonyflakeGenerator {
    /// Creates a generator with the default Sonyflake-style layout.
    ///
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier in `0..=65535`.
    ///
    /// # Returns
    ///
    /// A configured Sonyflake generator.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] when the default layout, start time, or lifetime
    /// is invalid, or when the current wall time has reached the exclusive
    /// expiration boundary.
    #[inline(always)]
    pub fn new(machine_id: u64) -> Result<Self, IdError> {
        Self::builder(machine_id).build()
    }

    /// Creates a configurable builder for a machine identifier.
    #[inline(always)]
    pub fn builder(machine_id: u64) -> SonyflakeGeneratorBuilder {
        SonyflakeGeneratorBuilder::new(machine_id)
    }

    /// Creates a public generator from a validated core and timer.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<SonyflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: BlockingSnowflake::new(core, timer),
        }
    }

    /// Returns the configured Sonyflake layout.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::SonyflakeGenerator;
    /// let generator = SonyflakeGenerator::new(7).expect("valid machine");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub const fn layout(&self) -> &SonyflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured elapsed-time origin.
    #[must_use]
    #[inline(always)]
    pub const fn start_time(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }
}

impl IdGenerator<u64> for SonyflakeGenerator {
    /// Generates the next Sonyflake-style ID.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns an allocation error or [`IdError::WaitFailed`] when the timer
    /// cannot complete a retry wait.
    #[inline(always)]
    fn generate(&self) -> Result<u64, IdError> {
        self.inner.generate()
    }
}
