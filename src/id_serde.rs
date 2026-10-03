// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Optional serde integration for [`crate::Id`].

use crate::Id;

impl serde::Serialize for Id {
    /// Serializes an identifier as decimal text for human-readable formats
    /// and as an unsigned 64-bit value for compact formats.
    ///
    /// # Type Parameters
    ///
    /// * `S` - Serializer that selects and encodes the output representation.
    ///
    /// # Parameters
    ///
    /// * `serializer` - Destination serializer for the identifier.
    ///
    /// # Returns
    ///
    /// The serializer's success value after writing the selected
    /// representation.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if writing the identifier fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_id::Id;
    ///
    /// let id = Id::new(42);
    /// assert_eq!(serde_json::to_string(&id).unwrap(), "\"42\"");
    /// ```
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
    /// Deserializes decimal text or an unsigned integer for human-readable
    /// formats and an unsigned 64-bit value for compact formats.
    ///
    /// # Type Parameters
    ///
    /// * `'de` - Lifetime of data borrowed from the deserializer.
    /// * `D` - Deserializer providing the input representation.
    ///
    /// # Parameters
    ///
    /// * `deserializer` - Source of the identifier representation.
    ///
    /// # Returns
    ///
    /// The identifier decoded from decimal text or an unsigned integer.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error for malformed decimal text, negative
    /// integers, or an input representation that cannot be decoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_id::Id;
    ///
    /// let from_text: Id = serde_json::from_str("\"42\"").unwrap();
    /// let from_integer: Id = serde_json::from_str("42").unwrap();
    /// assert_eq!(from_text, Id::new(42));
    /// assert_eq!(from_integer, Id::new(42));
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            /// Parses human-readable unsigned decimal identifiers.
            struct IdVisitor;

            impl<'de> serde::de::Visitor<'de> for IdVisitor {
                /// Identifier value produced by this visitor.
                type Value = Id;

                /// Describes the accepted human-readable identifier inputs.
                ///
                /// # Parameters
                ///
                /// * `formatter` - Destination for the expected input
                ///   description.
                ///
                /// # Returns
                ///
                /// Formatting success after writing the accepted input
                /// description.
                ///
                /// # Errors
                ///
                /// Returns the formatter's error if writing the description
                /// fails.
                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str("an unsigned decimal identifier string or integer")
                }

                /// Parses borrowed decimal text as an identifier.
                ///
                /// # Type Parameters
                ///
                /// * `E` - Deserializer error type.
                ///
                /// # Parameters
                ///
                /// * `value` - Decimal bytes borrowed from the input.
                ///
                /// # Returns
                ///
                /// The parsed identifier.
                ///
                /// # Errors
                ///
                /// Returns `E` when `value` is invalid or exceeds the `u64`
                /// range.
                fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_str(value)
                }

                /// Parses decimal text as an identifier.
                ///
                /// # Type Parameters
                ///
                /// * `E` - Deserializer error type.
                ///
                /// # Parameters
                ///
                /// * `value` - Decimal text supplied by the deserializer.
                ///
                /// # Returns
                ///
                /// The parsed identifier.
                ///
                /// # Errors
                ///
                /// Returns `E` when `value` is invalid or exceeds the `u64`
                /// range.
                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    value.parse().map_err(E::custom)
                }

                /// Parses owned decimal text as an identifier.
                ///
                /// # Type Parameters
                ///
                /// * `E` - Deserializer error type.
                ///
                /// # Parameters
                ///
                /// * `value` - Owned decimal text supplied by the deserializer.
                ///
                /// # Returns
                ///
                /// The parsed identifier.
                ///
                /// # Errors
                ///
                /// Returns `E` when `value` is invalid or exceeds the `u64`
                /// range.
                fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_str(&value)
                }

                /// Wraps an unsigned integer as an identifier.
                ///
                /// # Type Parameters
                ///
                /// * `E` - Deserializer error type.
                ///
                /// # Parameters
                ///
                /// * `value` - Nonnegative integer supplied by the
                ///   deserializer.
                ///
                /// # Returns
                ///
                /// The identifier containing `value`.
                fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Id::from(value))
                }

                /// Converts a signed integer to an identifier when nonnegative.
                ///
                /// # Type Parameters
                ///
                /// * `E` - Deserializer error type.
                ///
                /// # Parameters
                ///
                /// * `value` - Signed integer supplied by the deserializer.
                ///
                /// # Returns
                ///
                /// The identifier containing `value` when it fits in `u64`.
                ///
                /// # Errors
                ///
                /// Returns `E` for negative values that cannot convert to
                /// `u64`.
                fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    u64::try_from(value).map(Id::from).map_err(E::custom)
                }
            }

            deserializer.deserialize_any(IdVisitor)
        } else {
            let value = <u64 as serde::Deserialize>::deserialize(deserializer)?;
            Ok(Id::from(value))
        }
    }
}
