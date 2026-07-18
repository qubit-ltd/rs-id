// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous contract for ID generators.

use crate::{
    IdError,
    IdGenerationFuture,
};

/// Generates identifiers asynchronously.
///
/// The output and error types are trait parameters so applications can inject
/// a generator through an object-safe boundary such as
/// `Arc<dyn AsyncIdGenerator<String>>`. The error type defaults to
/// [`IdError`]. Implementations that mutate allocation state must synchronize
/// that state internally because generation uses a shared reference and may be
/// called concurrently.
pub trait AsyncIdGenerator<T: Send + 'static, E = IdError>:
    Send + Sync
{
    /// Generates the next identifier asynchronously.
    ///
    /// Implementations should yield while waiting for time or other external
    /// progress and must not block an asynchronous executor thread.
    ///
    /// # Returns
    ///
    /// A future that resolves to the next generated identifier.
    ///
    /// # Errors
    ///
    /// The returned future resolves to `E` when the implementation cannot
    /// generate an identifier.
    fn generate_async(&self) -> IdGenerationFuture<'_, T, E>;
}
