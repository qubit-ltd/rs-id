// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous classic 41/10/12 Snowflake generator.

use std::sync::Arc;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::internal::{
    BlockingSnowflake,
    SnowflakeCore,
};
use super::{
    SnowflakeGeneratorBuilder,
    SnowflakeLayout,
};
use crate::{
    IdError,
    IdGenerator,
};

/// Generates classic Snowflake IDs with 41 timestamp, 10 node, and 12
/// sequence bits.
///
/// The generator is thread-safe. One shared live instance never returns the
/// same ID twice, provided its node identifier is exclusive within the ID
/// namespace. [`IdGenerator::generate`] blocks when sequence capacity is
/// exhausted until the injected timer allows wall time to advance.
#[must_use]
pub struct SnowflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<SnowflakeLayout>,
}

impl SnowflakeGenerator {
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
    /// Returns [`IdError::NodeOutOfRange`] when `node_id` does not fit the
    /// 10-bit node field, [`IdError::ExpirationTimeOverflow`] when the lifetime
    /// boundary cannot be represented, or [`IdError::GeneratorExpired`] when
    /// the current wall time has reached that boundary.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
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
    pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(node_id)
    }

    /// Creates a public generator from a validated core and timer.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<SnowflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: BlockingSnowflake::new(core, timer),
        }
    }

    /// Returns the configured classic Snowflake layout.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::SnowflakeGenerator;
    /// let generator = SnowflakeGenerator::new(7).expect("valid node");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub const fn layout(&self) -> &SnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured timestamp origin.
    #[must_use]
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }
}

impl IdGenerator<u64> for SnowflakeGenerator {
    /// Generates the next classic Snowflake ID.
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
