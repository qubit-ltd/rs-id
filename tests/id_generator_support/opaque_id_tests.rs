// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines an opaque ID fixture without a `Display` implementation.

/// Opaque test ID whose formatting belongs to its generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpaqueId {
    /// Numeric payload used by the fixture formatter.
    pub(crate) value: u64,
}
