// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Domain value for generated unsigned 64-bit identifiers.

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// A generated identifier backed by an unsigned 64-bit value.
///
/// # Examples
///
/// ```
/// use qubit_id::Id;
///
/// let id = Id::new(42);
/// assert_eq!(id.value(), 42);
/// assert_eq!(id.to_string(), "42");
/// assert_eq!(id.to_padded_decimal(), "00000000000000000042");
/// ```
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct Id(
    /// Unsigned integer value represented by this identifier.
    u64,
);

impl Id {
    /// Creates an identifier from its underlying value.
    ///
    /// # Parameters
    ///
    /// * `value` - Unsigned integer to store in the identifier.
    ///
    /// # Returns
    ///
    /// An identifier containing `value`.
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying unsigned 64-bit value.
    ///
    /// # Returns
    ///
    /// The integer stored in this identifier.
    #[must_use]
    #[inline]
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Returns the identifier as a zero-padded, 20-digit decimal string.
    ///
    /// This representation preserves numeric order under lexicographic
    /// comparison and is intended for persistent text keys. [`fmt::Display`]
    /// keeps its unpadded decimal representation.
    ///
    /// # Returns
    ///
    /// The identifier formatted as exactly 20 decimal digits, with leading
    /// zeroes when needed.
    #[must_use]
    pub fn to_padded_decimal(self) -> String {
        format!("{:020}", self.value())
    }
}

impl From<u64> for Id {
    /// Wraps an unsigned 64-bit value as an identifier.
    ///
    /// # Parameters
    ///
    /// * `value` - Integer value to wrap.
    ///
    /// # Returns
    ///
    /// An identifier containing `value`.
    #[inline]
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Id> for u64 {
    /// Extracts the underlying unsigned 64-bit value.
    ///
    /// # Parameters
    ///
    /// * `id` - Identifier whose stored integer is extracted.
    ///
    /// # Returns
    ///
    /// The integer stored in `id`.
    #[inline]
    fn from(id: Id) -> Self {
        id.value()
    }
}

impl fmt::Display for Id {
    /// Formats the identifier as unsigned decimal text.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter receiving the decimal digits.
    ///
    /// # Returns
    ///
    /// Success when the formatter accepts all decimal digits.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if writing the decimal digits fails.
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for Id {
    /// Error returned when decimal text cannot be parsed as a `u64`.
    type Err = ParseIntError;

    /// Parses an unsigned decimal identifier.
    ///
    /// # Parameters
    ///
    /// * `value` - Decimal text to parse.
    ///
    /// # Returns
    ///
    /// The identifier represented by the decimal text.
    ///
    /// # Errors
    ///
    /// Returns the standard integer parsing error when `value` is empty,
    /// contains non-decimal characters, or exceeds the `u64` range.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self::new)
    }
}

impl TryFrom<&str> for Id {
    /// Error returned when borrowed decimal text cannot be parsed as a `u64`.
    type Error = ParseIntError;

    /// Parses an unsigned decimal identifier from borrowed text.
    ///
    /// # Parameters
    ///
    /// * `value` - Borrowed decimal text to parse.
    ///
    /// # Returns
    ///
    /// The identifier represented by the text.
    ///
    /// # Errors
    ///
    /// Returns the standard integer parsing error for invalid or overflowing
    /// decimal text.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
