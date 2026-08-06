// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Optional serde integration for [`crate::Id`].
// qubit-style: allow source-test-pair

use crate::Id;

impl serde::Serialize for Id {
    /// Serializes an identifier as decimal text for human-readable formats
    /// and as an unsigned 64-bit value for compact formats.
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_u64(self.value())
        }
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    /// Deserializes decimal text for human-readable formats and an unsigned
    /// 64-bit value for compact formats.
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let value =
                <String as serde::Deserialize>::deserialize(deserializer)?;
            value.parse().map_err(serde::de::Error::custom)
        } else {
            let value = <u64 as serde::Deserialize>::deserialize(deserializer)?;
            Ok(Id::from(value))
        }
    }
}
