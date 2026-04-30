/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! ID ordering mode for Java-compatible Qubit snowflake IDs.

/// Ordering mode encoded in a Qubit snowflake ID.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IdMode {
    /// Timestamp bits are stored in normal order, producing time-ordered IDs.
    Sequential,
    /// Timestamp bits are reversed, spreading adjacent timestamps across the ID space.
    Spread,
}

impl IdMode {
    /// Returns the one-bit ordinal used by the Java implementation.
    ///
    /// # Returns
    /// `0` for [`IdMode::Sequential`] and `1` for [`IdMode::Spread`].
    pub const fn ordinal(self) -> u64 {
        match self {
            Self::Sequential => 0,
            Self::Spread => 1,
        }
    }

    /// Decodes an ID mode from a one-bit value.
    ///
    /// # Parameters
    /// - `bit`: Encoded one-bit mode value.
    ///
    /// # Returns
    /// [`IdMode::Sequential`] for `0`; [`IdMode::Spread`] for every non-zero
    /// value after masking by callers.
    pub const fn from_bit(bit: u64) -> Self {
        if bit == 0 {
            Self::Sequential
        } else {
            Self::Spread
        }
    }
}
