// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Non-blocking contract for ID generators.

use crate::GenerationAttempt;

/// Attempts to allocate an identifier without sleeping or awaiting.
///
/// A successful [`GenerationAttempt::Generated`] reserves the identifier. A
/// [`GenerationAttempt::RetryAfter`] result leaves the generator usable and
/// tells the caller when another attempt can make progress. Implementations
/// must not block on clocks, timers, or external coordination.
pub trait TryIdGenerator: Send + Sync {
    /// Value produced by this generator.
    type Output;
    /// Error returned by this generator.
    type Error;

    /// Attempts one non-blocking allocation.
    ///
    /// # Returns
    ///
    /// A generated identifier or a retry decision.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the allocation cannot recover by waiting.
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error>;
}

impl<G> TryIdGenerator for std::sync::Arc<G>
where
    G: TryIdGenerator + ?Sized,
{
    type Output = G::Output;
    type Error = G::Error;

    /// Delegates one non-blocking allocation to the shared generator.
    #[inline(always)]
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error> {
        self.as_ref().try_generate()
    }
}
