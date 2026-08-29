// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! ID ordering mode for Qubit snowflake IDs.

/// Ordering mode encoded in a Qubit snowflake ID.
///
/// # Must use
///
/// Query results on ID configuration values must not be silently discarded.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use qubit_id::{IdMode, TimestampPrecision};
///
/// IdMode::Sequential.ordinal();
/// TimestampPrecision::Millisecond.sequence_bits();
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[must_use]
pub enum IdMode {
    /// Timestamp bits are stored in normal order, producing time-ordered IDs.
    Sequential,
    /// Timestamp bits are reversed, spreading adjacent timestamps across the ID
    /// space.
    ///
    /// This mode is useful for public identifiers such as order numbers. With
    /// sequential timestamp bits, users can compare identifiers issued at
    /// different times and make rough inferences about ordering or activity
    /// volume. Spreading the timestamp bits breaks that direct lexical and
    /// numeric relationship between adjacent time slices.
    ///
    /// Spread is reversible obfuscation, not encryption or a confidentiality
    /// boundary. IDs created within the same time slice still have increasing
    /// sequence values, and anyone who knows the layout can decode the original
    /// timestamp.
    Spread,
}

impl IdMode {
    /// Decodes an ID mode from a one-bit value.
    ///
    /// # Parameters
    ///
    /// * `bit` - Encoded one-bit mode value.
    ///
    /// # Returns
    ///
    /// [`IdMode::Sequential`] for `0`; [`IdMode::Spread`] for every non-zero
    /// value after masking by callers.
    #[inline]
    pub const fn from_bit(bit: u64) -> Self {
        if bit == 0 { Self::Sequential } else { Self::Spread }
    }

    /// Returns the one-bit ordinal used by the Qubit layout.
    ///
    /// # Returns
    ///
    /// `0` for [`IdMode::Sequential`] and `1` for [`IdMode::Spread`].
    #[must_use]
    #[inline(always)]
    pub const fn ordinal(self) -> u64 {
        match self {
            Self::Sequential => 0,
            Self::Spread => 1,
        }
    }
}
