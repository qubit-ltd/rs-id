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

/// Object-safe future returned by an asynchronous ID generator.
///
/// The future may borrow its generator for `'a` and is safe to move between
/// executor threads.
///
/// # Type Parameters
///
/// * `'a` - Maximum lifetime of any borrow held by the future.
/// * `T` - Identifier value produced when the future completes successfully.
/// * `E` - Error value produced when generation fails.
///
/// # Examples
///
/// ```
/// use std::convert::Infallible;
///
/// use qubit_id::IdGenerationFuture;
///
/// let future: IdGenerationFuture<'static, u64, Infallible> =
///     Box::pin(async { Ok(42) });
/// let runtime = tokio::runtime::Builder::new_current_thread()
///     .build()
///     .unwrap();
/// assert_eq!(runtime.block_on(future).unwrap(), 42);
/// ```
pub type IdGenerationFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;
