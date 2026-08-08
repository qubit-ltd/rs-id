// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Blocking generation capability for ID generators.

use std::sync::Arc;

use crate::Id;
use crate::IdGenerationError;

/// Generates identifiers synchronously, blocking when necessary.
///
/// Implementations may wait for time to advance or for another retryable
/// condition to clear. Use [`crate::TryIdGenerator`] when callers must retain
/// control over retry scheduling.
pub trait IdGenerator<Output = Id, Error = IdGenerationError>:
    Send + Sync
{
    /// Generates the next identifier.
    ///
    /// # Returns
    ///
    /// The next generated identifier.
    ///
    /// # Errors
    ///
    /// Returns `Error` when the implementation cannot generate an identifier.
    fn generate(&self) -> Result<Output, Error>;
}

impl<Output, Error, G> IdGenerator<Output, Error> for Arc<G>
where
    G: IdGenerator<Output, Error> + ?Sized,
{
    /// Delegates identifier generation to the shared generator.
    ///
    /// # Returns
    ///
    /// The next identifier returned by the wrapped generator.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the wrapped generator.
    #[inline(always)]
    fn generate(&self) -> Result<Output, Error> {
        self.as_ref().generate()
    }
}
