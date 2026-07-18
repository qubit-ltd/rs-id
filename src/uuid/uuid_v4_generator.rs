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
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
    IdGenerator,
};

/// Generates UUID v4 values as native [`u128`] identifiers.
///
/// UUID uniqueness is probabilistic. This stateless generator is safe to share
/// across threads and tasks.
#[derive(Debug, Default, Clone, Copy)]
#[must_use]
pub struct UuidV4Generator;

impl UuidV4Generator {
    /// Creates a UUID v4 numeric generator.
    ///
    /// # Returns
    ///
    /// A stateless UUID v4 generator.
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }

    /// Generates a UUID v4 through an allocation-free concrete future.
    ///
    /// Use [`AsyncIdGenerator`] when object-safe dynamic dispatch is required.
    ///
    /// # Returns
    ///
    /// A future that completes on its first poll with a random UUID v4 value.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::RandomSourceFailed`] when the operating-system
    /// random source cannot provide UUID bytes.
    #[inline(always)]
    pub async fn generate_async(&self) -> Result<u128, IdError> {
        generate_uuid_v4().map(|uuid| uuid.as_u128())
    }
}

impl IdGenerator<u128> for UuidV4Generator {
    /// Generates a UUID v4 as a native `u128`.
    ///
    /// # Returns
    ///
    /// The next random UUID v4 value.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::RandomSourceFailed`] when the operating-system
    /// random source cannot provide UUID bytes.
    #[inline(always)]
    fn generate(&self) -> Result<u128, IdError> {
        generate_uuid_v4().map(|uuid| uuid.as_u128())
    }
}

impl AsyncIdGenerator<u128> for UuidV4Generator {
    /// Generates a UUID v4 through an immediately ready future.
    ///
    /// # Returns
    ///
    /// A future that completes on its first poll with a random UUID v4 value.
    ///
    /// # Errors
    ///
    /// The future resolves to [`IdError::RandomSourceFailed`] when the
    /// operating-system random source cannot provide UUID bytes.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u128> {
        Box::pin(UuidV4Generator::generate_async(self))
    }
}
