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
            serializer.collect_str(self)
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
            struct IdVisitor;

            impl<'de> serde::de::Visitor<'de> for IdVisitor {
                type Value = Id;

                fn expecting(
                    &self,
                    formatter: &mut std::fmt::Formatter<'_>,
                ) -> std::fmt::Result {
                    formatter.write_str("an unsigned decimal identifier string")
                }

                fn visit_borrowed_str<E>(
                    self,
                    value: &'de str,
                ) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_str(value)
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    value.parse().map_err(E::custom)
                }

                fn visit_string<E>(
                    self,
                    value: String,
                ) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_str(&value)
                }
            }

            deserializer.deserialize_str(IdVisitor)
        } else {
            let value = <u64 as serde::Deserialize>::deserialize(deserializer)?;
            Ok(Id::from(value))
        }
    }
}
