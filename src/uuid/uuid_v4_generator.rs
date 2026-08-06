// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standards-compliant UUID v4 numeric generator.

use super::internal::generate_uuid_v4;
use crate::{
    IdGenerationError,
    IdGenerator,
};

/// Generates UUID v4 values as [`uuid::Uuid`] identifiers.
///
/// UUID uniqueness is probabilistic. This stateless generator is safe to share
/// across threads and tasks.
///
/// This type intentionally exposes only [`IdGenerator`]. Applications that
/// generate UUIDs from an async runtime must choose the runtime-specific
/// blocking boundary explicitly.
///
/// ```compile_fail
/// use qubit_id::{AsyncIdGenerator, UuidV4Generator};
///
/// fn require_async<G: AsyncIdGenerator<Output = uuid::Uuid>>(_generator: &G) {}
///
/// require_async(&UuidV4Generator::new());
/// ```
#[derive(Debug, Default, Clone, Copy)]
#[must_use]
pub struct UuidV4Generator;

impl UuidV4Generator {
    /// Creates a UUID v4 generator.
    ///
    /// # Returns
    ///
    /// A stateless UUID v4 generator.
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }

    /// Generates a UUID v4 value.
    ///
    /// This inherent method is convenient for concrete callers. Use
    /// [`IdGenerator`] when an object-safe dynamic-dispatch boundary is needed.
    ///
    /// # Returns
    ///
    /// The next random UUID v4 value.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::RandomSourceFailed`] when the
    /// operating-system random source cannot provide UUID bytes.
    #[inline(always)]
    pub fn generate(&self) -> Result<uuid::Uuid, IdGenerationError> {
        generate_uuid_v4()
    }
}

impl IdGenerator for UuidV4Generator {
    type Output = uuid::Uuid;
    type Error = IdGenerationError;

    /// Generates a UUID v4 as a [`uuid::Uuid`].
    ///
    /// # Returns
    ///
    /// The next random UUID v4 value.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::RandomSourceFailed`] when the
    /// operating-system random source cannot provide UUID bytes.
    #[inline(always)]
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        UuidV4Generator::generate(self)
    }
}
