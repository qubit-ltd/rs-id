// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Non-blocking contract for ID generators.

use std::sync::Arc;

use crate::{
    GenerationAttempt,
    IdGenerator,
};

/// Attempts to allocate an identifier without sleeping or awaiting.
///
/// A successful [`GenerationAttempt::Generated`] reserves the identifier. A
/// [`GenerationAttempt::RetryAfter`] result leaves the generator usable and
/// tells the caller when another attempt can make progress. Implementations
/// must not block on clocks, timers, or external coordination.
pub trait TryIdGenerator: IdGenerator {
    /// Attempts one non-blocking allocation.
    ///
    /// # Returns
    ///
    /// A generated identifier or a retry decision.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerator::Error`] when the allocation cannot recover by
    /// waiting.
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error>;
}

impl<G> TryIdGenerator for Arc<G>
where
    G: TryIdGenerator + ?Sized,
{
    /// Delegates one non-blocking allocation to the shared generator.
    #[inline(always)]
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error> {
        self.as_ref().try_generate()
    }
}
