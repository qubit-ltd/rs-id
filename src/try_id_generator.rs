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

/// Attempts to allocate an identifier without sleeping or awaiting for an
/// external condition to change. An implementation may use a short synchronous
/// critical section; this trait does not promise lock-free or wait-free calls
/// or a latency bound.
///
/// A successful [`GenerationAttempt::Generated`] reserves the identifier. A
/// [`GenerationAttempt::RetryAfter`] result leaves the generator usable and
/// tells the caller when another attempt can make progress. Implementations
/// must not wait for clocks, timers, or external coordination to make progress.
/// Implementations may briefly synchronize access to allocation state.
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
    /// Attempts one allocation without waiting for time or another external
    /// condition to progress. The implementation may briefly acquire a
    /// synchronous lock.
    ///
    /// # Returns
    ///
    /// A generated identifier or a retry decision.
    ///
    /// # Errors
    ///
    /// Returns `Error` when the current allocation attempt fails. Recovery
    /// depends on the implementation and error; an error does not necessarily
    /// make the generator permanently unusable.
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
