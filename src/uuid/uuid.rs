// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Domain value for UUID bit patterns.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid as RawUuid;

/// A UUID value backed by its 128-bit representation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct Uuid(u128);

impl Uuid {
    /// Creates a UUID from its underlying bit pattern.
    #[inline(always)]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Returns the underlying 128-bit UUID value.
    #[inline(always)]
    pub const fn value(self) -> u128 {
        self.0
    }
}

impl From<u128> for Uuid {
    /// Wraps a 128-bit value as a UUID.
    #[inline(always)]
    fn from(value: u128) -> Self {
        Self::new(value)
    }
}

impl From<Uuid> for u128 {
    /// Extracts the underlying UUID bit pattern.
    #[inline(always)]
    fn from(uuid: Uuid) -> Self {
        uuid.value()
    }
}

impl fmt::Display for Uuid {
    /// Formats the UUID as lowercase hyphenated text.
    #[inline(always)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        RawUuid::from_u128(self.0).hyphenated().fmt(formatter)
    }
}

impl FromStr for Uuid {
    type Err = uuid::Error;

    /// Parses UUID text accepted by the `uuid` crate.
    ///
    /// # Errors
    ///
    /// Returns [`uuid::Error`] when `value` is not a valid UUID string.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        RawUuid::parse_str(value).map(|uuid| Self::new(uuid.as_u128()))
    }
}

impl TryFrom<&str> for Uuid {
    type Error = uuid::Error;

    /// Parses UUID text from a borrowed string slice.
    ///
    /// # Errors
    ///
    /// Returns [`uuid::Error`] when `value` is not a valid UUID string.
    #[inline(always)]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
