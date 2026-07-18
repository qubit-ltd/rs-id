// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous contract for ID generators.

use crate::IdError;

/// Generates identifiers synchronously.
///
/// The output type is a trait parameter so applications can inject a generator
/// through an object-safe boundary such as `Arc<dyn IdGenerator<u64>>`.
/// Implementations that mutate allocation state must synchronize that state
/// internally because generation uses a shared reference and may be called
/// concurrently.
pub trait IdGenerator<T: Send + 'static>: Send + Sync {
    /// Generates the next identifier.
    ///
    /// This method may block when an implementation must wait for time to
    /// advance or another retryable condition to clear.
    ///
    /// # Returns
    ///
    /// The next generated identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the implementation cannot generate an
    /// identifier.
    fn generate(&self) -> Result<T, IdError>;
}
