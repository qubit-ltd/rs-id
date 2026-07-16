// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Mica-style random UUID-like ID generation.
//!
//! The formatting approach follows Mica's fast UUID helper and unsigned
//! hexadecimal formatter from [`StringUtil`], plus the related
//! [Mica UUID benchmark notes].
//!
//! [`StringUtil`]: https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L335
//! [Mica UUID benchmark notes]: https://github.com/lets-mica/mica-jmh/wiki/uuid

use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

/// Lowercase hexadecimal digits used by the Mica UUID-like formatter.
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Mask for extracting one hexadecimal digit from the low four bits.
///
/// A hexadecimal digit is a 4-bit nibble. After shifting the source value by a
/// multiple of four bits, this mask keeps only the current digit. This mirrors
/// the Java helper's `MASK = HEX_RADIX - 1` constant in Mica's
/// `StringUtil::formatUnsignedLong`.
const HEX_DIGIT_MASK: u128 = 0x0f;

/// Mica-style UUID-like random ID generator.
///
/// This generator is only a random number generator that mimics the canonical
/// UUID text shape. It produces 128 random bits and formats them as lowercase
/// UUID-like text, but it does not rewrite RFC UUID version or variant bits.
/// Therefore it should not be treated as a standards-compliant UUID v4
/// generator.
///
/// The generator has no synchronized mutable allocation state. Each successful
/// call reads 128 random bits from the operating system. Uniqueness is
/// therefore probabilistic, with a theoretical collision possibility, rather
/// than the deterministic per-instance guarantee of the Snowflake-family
/// generators.
///
/// # Origin
/// The formatting approach is based on Mica's fast UUID utility and
/// `formatUnsignedLong` helper:
/// <https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L348>.
/// The Java source also points to Mica's UUID benchmark notes:
/// <https://github.com/lets-mica/mica-jmh/wiki/uuid>.
#[derive(Debug, Default, Clone, Copy)]
pub struct MicaUuidLikeGenerator;

impl MicaUuidLikeGenerator {
    /// Creates a Mica-style UUID-like generator.
    ///
    /// # Returns
    ///
    /// A Mica-style UUID-like generator.
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }

    /// Formats a `u128` as canonical lowercase UUID-like text.
    ///
    /// # Arguments
    ///
    /// * `value` - 128-bit ID value.
    ///
    /// # Returns
    ///
    /// UUID-like text in `8-4-4-4-12` lowercase hexadecimal form.
    #[inline]
    pub fn format_uuid_like(value: u128) -> String {
        let mut output = String::with_capacity(36);
        push_hex(&mut output, value >> 96, 8);
        output.push('-');
        push_hex(&mut output, value >> 80, 4);
        output.push('-');
        push_hex(&mut output, value >> 64, 4);
        output.push('-');
        push_hex(&mut output, value >> 48, 4);
        output.push('-');
        push_hex(&mut output, value, 12);
        output
    }

    /// Formats a `u128` as compact lowercase UUID-like text.
    ///
    /// # Arguments
    ///
    /// * `value` - 128-bit ID value.
    ///
    /// # Returns
    ///
    /// UUID-like text as 32 lowercase hexadecimal digits without separators.
    #[inline]
    pub fn format_simple_uuid_like(value: u128) -> String {
        let mut output = String::with_capacity(32);
        push_hex(&mut output, value, 32);
        output
    }
}

impl IdGenerator for MicaUuidLikeGenerator {
    type Id = u128;
    type Error = IdError;

    /// Performs one random UUID-like allocation attempt without sleeping.
    ///
    /// # Returns
    ///
    /// A generated random 128-bit value.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::RandomSourceUnavailable`] when the operating system
    /// random source fails.
    #[inline(always)]
    fn try_next_id(&self) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        generate_id().map(GenerationOutcome::Generated)
    }

    /// Generates the next random 128-bit UUID-like value.
    ///
    /// # Returns
    ///
    /// A generated random 128-bit value.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::RandomSourceUnavailable`] when the operating system
    /// random source fails.
    #[inline(always)]
    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        generate_id()
    }

    /// Formats an ID value with canonical UUID separators.
    ///
    /// # Arguments
    ///
    /// * `id` - Random ID value to format.
    ///
    /// # Returns
    ///
    /// Canonical lowercase UUID-like text.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        Self::format_uuid_like(*id)
    }
}

/// Generates a canonical lowercase UUID-like random string.
///
/// # Returns
///
/// UUID-like text in `8-4-4-4-12` lowercase hexadecimal form.
///
/// # Errors
///
/// Returns [`IdError::RandomSourceUnavailable`] when the operating system
/// random source cannot fill 16 bytes.
#[inline(always)]
pub fn fast_uuid_like() -> Result<String, IdError> {
    MicaUuidLikeGenerator::new().next_string()
}

/// Generates a compact lowercase UUID-like random string.
///
/// # Returns
///
/// UUID-like text as 32 lowercase hexadecimal digits without separators.
///
/// # Errors
///
/// Returns [`IdError::RandomSourceUnavailable`] when the operating system
/// random source cannot fill 16 bytes.
#[inline(always)]
pub fn fast_simple_uuid_like() -> Result<String, IdError> {
    let id = MicaUuidLikeGenerator::new().next_id()?;
    Ok(MicaUuidLikeGenerator::format_simple_uuid_like(id))
}

/// Appends fixed-width lowercase hexadecimal digits to a string.
///
/// # Arguments
///
/// * `output` - Destination string.
/// * `value` - Source value; only the lowest `digits * 4` bits are used.
/// * `digits` - Number of hexadecimal digits to append, at most 32.
///
/// # Panics
///
/// Panics when `digits` exceeds the 32 hexadecimal digits representable by a
/// `u128`.
fn push_hex(output: &mut String, value: u128, digits: usize) {
    for index in (0..digits).rev() {
        let nibble = ((value >> (index * 4)) & HEX_DIGIT_MASK) as usize;
        output.push(char::from(HEX[nibble]));
    }
}

/// Reads one random UUID-like value from the operating system.
///
/// # Returns
///
/// A uniformly random 128-bit value.
///
/// # Errors
///
/// Returns [`IdError::RandomSourceUnavailable`] when the random source fails.
fn generate_id() -> Result<u128, IdError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|source| IdError::RandomSourceUnavailable { source })?;
    Ok(u128::from_be_bytes(bytes))
}
