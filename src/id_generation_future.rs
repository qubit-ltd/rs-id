// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Boxed future returned by asynchronous ID generators.

use std::future::Future;
use std::pin::Pin;

use crate::IdError;

/// Object-safe future returned by an asynchronous ID generator.
///
/// The future may borrow its generator for `'a` and is safe to move between
/// executor threads. `E` defaults to [`IdError`] for built-in generators.
pub type IdGenerationFuture<'a, T, E = IdError> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;
