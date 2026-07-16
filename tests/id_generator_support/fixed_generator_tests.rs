// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a deterministic ID generator fixture.

use std::convert::Infallible;

use qubit_id::{
    GenerationOutcome,
    IdGenerator,
};

use super::OpaqueId;

/// Generator that always returns one opaque ID value.
pub(crate) struct FixedGenerator {
    /// Numeric payload returned by every attempt.
    value: u64,
}

impl FixedGenerator {
    /// Creates a fixed generator for `value`.
    ///
    /// # Arguments
    ///
    /// * `value` - Numeric payload returned by every attempt.
    ///
    /// # Returns
    ///
    /// A generator containing `value`.
    #[inline]
    pub(crate) const fn new(value: u64) -> Self {
        Self { value }
    }
}

impl IdGenerator for FixedGenerator {
    type Id = OpaqueId;
    type Error = Infallible;

    /// Returns the fixture's fixed opaque ID without blocking.
    ///
    /// # Returns
    ///
    /// The fixed opaque ID as a successful generation outcome.
    #[inline(always)]
    fn try_next_id(&self) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        Ok(GenerationOutcome::Generated(OpaqueId { value: self.value }))
    }

    /// Returns the fixture's fixed opaque ID.
    ///
    /// # Returns
    ///
    /// The fixed opaque ID.
    #[inline(always)]
    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        Ok(OpaqueId { value: self.value })
    }

    /// Formats the opaque fixture value.
    ///
    /// # Arguments
    ///
    /// * `id` - Opaque ID to format.
    ///
    /// # Returns
    ///
    /// Text in the fixture's `opaque:<value>` format.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        format!("opaque:{}", id.value)
    }
}
