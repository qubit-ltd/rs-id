// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared standards-compliant UUID v4 generation.

use ::uuid::Uuid;

/// Generates one UUID v4 value from the operating-system random source.
///
/// # Returns
///
/// A standards-compliant random UUID v4 value.
///
/// # Panics
///
/// Panics when the operating-system random source is unavailable.
#[inline(always)]
pub(crate) fn generate_uuid_v4() -> Uuid {
    Uuid::new_v4()
}
