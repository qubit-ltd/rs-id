// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Common type contract for ID generators.

use std::sync::Arc;

/// Describes the output and error types shared by ID-generation capabilities.
///
/// This trait deliberately provides no generation method. Pair it with
/// [`crate::BlockingIdGenerator`], [`crate::TryIdGenerator`], or
/// [`crate::AsyncIdGenerator`] according to the caller's scheduling model.
/// Implementations that mutate allocation state must synchronize that state
/// internally because those capabilities use a shared reference and may be
/// called concurrently.
pub trait IdGenerator: Send + Sync {
    /// Value produced by this generator.
    type Output;
    /// Error returned by this generator.
    type Error;
}

impl<G> IdGenerator for Arc<G>
where
    G: IdGenerator + ?Sized,
{
    type Output = G::Output;
    type Error = G::Error;
}
