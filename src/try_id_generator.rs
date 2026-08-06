// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Non-blocking contract for ID generators.

use crate::{GenerationAttempt, IdError};

/// Attempts to allocate an identifier without sleeping or awaiting.
///
/// A successful [`GenerationAttempt::Generated`] reserves the identifier. A
/// [`GenerationAttempt::RetryAfter`] result leaves the generator usable and
/// tells the caller when another attempt can make progress. Implementations
/// must not block on clocks, timers, or external coordination.
pub trait TryIdGenerator<T, E = IdError>: Send + Sync {
    /// Attempts one non-blocking allocation.
    ///
    /// # Returns
    ///
    /// A generated identifier or a retry decision.
    ///
    /// # Errors
    ///
    /// Returns `E` when the allocation cannot recover by waiting.
    fn try_generate(&self) -> Result<GenerationAttempt<T>, E>;
}

impl<T, E, G> TryIdGenerator<T, E> for std::sync::Arc<G>
where
    G: TryIdGenerator<T, E> + ?Sized,
{
    /// Delegates one non-blocking allocation to the shared generator.
    #[inline(always)]
    fn try_generate(&self) -> Result<GenerationAttempt<T>, E> {
        self.as_ref().try_generate()
    }
}
