// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines an ID generator fixture that always fails.

use std::io::Error;

use qubit_id::{
    GenerationOutcome,
    IdGenerator,
};

use super::OpaqueId;

/// Generator that returns a stable error from every attempt.
pub(crate) struct FailingGenerator;

impl IdGenerator for FailingGenerator {
    type Id = OpaqueId;
    type Error = Error;

    /// Returns the fixture's stable generation error without blocking.
    ///
    /// # Returns
    ///
    /// This fixture never generates an ID.
    ///
    /// # Errors
    ///
    /// Always returns an [`Error`] with a stable fixture message.
    #[inline(always)]
    fn try_next_id(&self) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        Err(Error::other("fixture generation failed"))
    }

    /// Returns the fixture's stable generation error.
    ///
    /// # Returns
    ///
    /// This fixture never generates an ID.
    ///
    /// # Errors
    ///
    /// Always returns an [`Error`] with a stable fixture message.
    #[inline(always)]
    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        Err(Error::other("fixture generation failed"))
    }

    /// Formats an opaque fixture value if one is supplied directly.
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
