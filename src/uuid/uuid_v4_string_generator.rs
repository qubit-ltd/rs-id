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
    IdError,
    IdGenerator,
};

/// Generates canonical lowercase hyphenated UUID v4 strings.
///
/// UUID uniqueness is probabilistic. This stateless generator is safe to share
/// across threads and tasks.
///
/// This type intentionally exposes only [`IdGenerator`]. Applications that
/// generate UUIDs from an async runtime must choose the runtime-specific
/// blocking boundary explicitly.
///
/// ```compile_fail
/// use qubit_id::{AsyncIdGenerator, UuidV4StringGenerator};
///
/// fn require_async<G: AsyncIdGenerator<String>>(_generator: &G) {}
///
/// require_async(&UuidV4StringGenerator::new());
/// ```
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
    /// Returns [`IdError::RandomSourceFailed`] when the operating-system
    /// random source cannot provide UUID bytes.
    #[inline(always)]
    fn generate(&self) -> Result<String, IdError> {
        generate_uuid_v4().map(|uuid| uuid.hyphenated().to_string())
    }
}
