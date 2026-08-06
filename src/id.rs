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
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct Id(u64);

impl Id {
    /// Creates an identifier from its underlying value.
    #[inline(always)]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying unsigned 64-bit value.
    #[inline(always)]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for Id {
    /// Wraps an unsigned 64-bit value as an identifier.
    #[inline(always)]
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Id> for u64 {
    /// Extracts the underlying unsigned 64-bit value.
    #[inline(always)]
    fn from(id: Id) -> Self {
        id.value()
    }
}

impl fmt::Display for Id {
    /// Formats the identifier as unsigned decimal text.
    #[inline(always)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for Id {
    type Err = ParseIntError;

    /// Parses an unsigned decimal identifier.
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
    type Error = ParseIntError;

    /// Parses an unsigned decimal identifier from borrowed text.
    ///
    /// # Errors
    ///
    /// Returns the standard integer parsing error for invalid or overflowing
    /// decimal text.
    #[inline(always)]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Id {
    /// Serializes an identifier as decimal text for human-readable formats
    /// and as an unsigned 64-bit value for compact formats.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_u64(self.0)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Id {
    /// Deserializes decimal text for human-readable formats and an unsigned
    /// 64-bit value for compact formats.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(IdStringVisitor)
        } else {
            deserializer.deserialize_u64(IdU64Visitor)
        }
    }
}

#[cfg(feature = "serde")]
struct IdStringVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for IdStringVisitor {
    type Value = Id;

    /// Describes the decimal string accepted by this visitor.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an unsigned 64-bit identifier in decimal text")
    }

    /// Parses one decimal identifier string.
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value.parse().map_err(E::custom)
    }
}

#[cfg(feature = "serde")]
struct IdU64Visitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for IdU64Visitor {
    type Value = Id;

    /// Describes the compact unsigned integer accepted by this visitor.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an unsigned 64-bit identifier")
    }

    /// Wraps one compact unsigned identifier value.
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Id::from(value))
    }
}
