// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Non-blocking contract for ID generators.

use std::sync::Arc;

use crate::GenerationAttempt;
use crate::Id;
use crate::IdGenerationError;

/// Attempts to allocate an identifier without sleeping or awaiting.
///
/// A successful [`GenerationAttempt::Generated`] reserves the identifier. A
/// [`GenerationAttempt::RetryAfter`] result leaves the generator usable and
/// tells the caller when another attempt can make progress. Implementations
/// must not block on clocks, timers, or external coordination.
///
/// # Examples
///
/// ```
/// use std::convert::Infallible;
///
/// use qubit_id::{GenerationAttempt, TryIdGenerator};
///
/// struct ImmediateGenerator;
///
/// impl TryIdGenerator<u64, Infallible> for ImmediateGenerator {
///     fn try_generate(&self) -> Result<GenerationAttempt<u64>, Infallible> {
///         Ok(GenerationAttempt::Generated(42))
///     }
/// }
///
/// let attempt = ImmediateGenerator.try_generate().unwrap();
/// assert_eq!(attempt, GenerationAttempt::Generated(42));
/// ```
pub trait TryIdGenerator<Output = Id, Error = IdGenerationError>: Send + Sync {
    /// Attempts one non-blocking allocation.
    ///
    /// # Returns
    ///
    /// A generated identifier or a retry decision.
    ///
    /// # Errors
    ///
    /// Returns `Error` when the allocation cannot recover by waiting.
    fn try_generate(&self) -> Result<GenerationAttempt<Output>, Error>;
}

impl<Output, Error, G> TryIdGenerator<Output, Error> for Arc<G>
where
    G: TryIdGenerator<Output, Error> + ?Sized,
{
    /// Delegates one non-blocking allocation to the shared generator.
    #[inline]
    fn try_generate(&self) -> Result<GenerationAttempt<Output>, Error> {
        self.as_ref().try_generate()
    }
}
