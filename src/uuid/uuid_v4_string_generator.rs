// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standards-compliant UUID v4 string generator.

use super::internal::generate_uuid_v4;
use crate::{
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
    IdGenerator,
};

/// Generates canonical lowercase hyphenated UUID v4 strings.
///
/// UUID uniqueness is probabilistic. This stateless generator is safe to share
/// across threads and tasks.
#[derive(Debug, Default, Clone, Copy)]
#[must_use]
pub struct UuidV4StringGenerator;

impl UuidV4StringGenerator {
    /// Creates a UUID v4 string generator.
    ///
    /// # Returns
    ///
    /// A stateless UUID v4 string generator.
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }
}

impl IdGenerator<String> for UuidV4StringGenerator {
    /// Generates a canonical lowercase hyphenated UUID v4 string.
    ///
    /// # Returns
    ///
    /// The next random UUID v4 string.
    ///
    /// # Errors
    ///
    /// This implementation does not return a recoverable generation error.
    ///
    /// # Panics
    ///
    /// Panics when the operating-system random source is unavailable.
    #[inline(always)]
    fn generate(&self) -> Result<String, IdError> {
        Ok(generate_uuid_v4().hyphenated().to_string())
    }
}

impl AsyncIdGenerator<String> for UuidV4StringGenerator {
    /// Generates a canonical UUID v4 string through an immediately ready
    /// future.
    ///
    /// # Returns
    ///
    /// A future that completes on its first poll with a random UUID v4 string.
    ///
    /// # Errors
    ///
    /// The future does not resolve to a recoverable generation error.
    ///
    /// # Panics
    ///
    /// Polling the future panics when the operating-system random source is
    /// unavailable.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, String> {
        Box::pin(async { Ok(generate_uuid_v4().hyphenated().to_string()) })
    }
}
