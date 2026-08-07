// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Common type contract for ID generators.

use std::sync::Arc;

use crate::{
    Id,
    IdGenerationError,
};

/// Describes the output and error types shared by ID-generation capabilities.
///
/// `Output` defaults to [`Id`] and `Error` defaults to [`IdGenerationError`].
/// Specify either parameter explicitly when a generator uses different types.
///
/// This trait deliberately provides no generation method. Pair it with
/// [`crate::BlockingIdGenerator`], [`crate::TryIdGenerator`], or
/// [`crate::AsyncIdGenerator`] according to the caller's scheduling model.
/// Implementations that mutate allocation state must synchronize that state
/// internally because those capabilities use a shared reference and may be
/// called concurrently.
pub trait IdGenerator<Output = Id, Error = IdGenerationError>: Send + Sync {}

impl<Output, Error, G> IdGenerator<Output, Error> for Arc<G>
where
    G: IdGenerator<Output, Error> + ?Sized,
{
}
