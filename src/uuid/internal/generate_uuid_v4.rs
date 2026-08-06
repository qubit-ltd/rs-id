// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Shared standards-compliant UUID v4 generation.

use ::uuid::{
    Builder,
    Uuid,
};

use crate::IdGenerationError;

/// Generates one UUID v4 value from the operating-system random source.
///
/// # Returns
///
/// A standards-compliant random UUID v4 value.
///
/// # Errors
///
/// Returns [`IdGenerationError::RandomSourceFailed`] when the operating-system
/// random source cannot fill the UUID bytes.
#[inline(always)]
pub(crate) fn generate_uuid_v4() -> Result<Uuid, IdGenerationError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|source| IdGenerationError::RandomSourceFailed { source })?;
    Ok(Builder::from_random_bytes(bytes).into_uuid())
}
