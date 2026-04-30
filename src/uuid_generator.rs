/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Fast random UUID-shaped ID generation.

use crate::{IdError, IdGenerator};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Fast UUID-shaped random ID generator.
///
/// This generator matches the Java helper's performance-oriented behavior: it
/// produces 128 random bits and formats them as lowercase UUID text. It does
/// not rewrite version or variant bits.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl UuidGenerator {
    /// Creates a UUID generator.
    ///
    /// # Returns
    /// A UUID generator.
    pub const fn new() -> Self {
        Self
    }

    /// Formats a `u128` as canonical lowercase UUID text.
    ///
    /// # Parameters
    /// - `value`: 128-bit UUID value.
    ///
    /// # Returns
    /// UUID text in `8-4-4-4-12` lowercase hexadecimal form.
    pub fn format_uuid(value: u128) -> String {
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

    /// Formats a `u128` as compact lowercase UUID text.
    ///
    /// # Parameters
    /// - `value`: 128-bit UUID value.
    ///
    /// # Returns
    /// UUID text as 32 lowercase hexadecimal digits without separators.
    pub fn format_simple_uuid(value: u128) -> String {
        let mut output = String::with_capacity(32);
        push_hex(&mut output, value, 32);
        output
    }
}

impl IdGenerator<u128> for UuidGenerator {
    type Error = IdError;

    /// Generates the next random 128-bit UUID value.
    fn next_id(&self) -> Result<u128, Self::Error> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| IdError::RandomSourceUnavailable)?;
        Ok(u128::from_be_bytes(bytes))
    }

    /// Formats a UUID value with canonical UUID separators.
    fn format_id(&self, id: &u128) -> String {
        Self::format_uuid(*id)
    }
}

/// Generates a canonical lowercase UUID-shaped random string.
///
/// # Returns
/// UUID text in `8-4-4-4-12` lowercase hexadecimal form.
///
/// # Errors
/// Returns [`IdError::RandomSourceUnavailable`] when the operating system
/// random source cannot fill 16 bytes.
pub fn fast_uuid() -> Result<String, IdError> {
    UuidGenerator::new().next_string()
}

/// Generates a compact lowercase UUID-shaped random string.
///
/// # Returns
/// UUID text as 32 lowercase hexadecimal digits without separators.
///
/// # Errors
/// Returns [`IdError::RandomSourceUnavailable`] when the operating system
/// random source cannot fill 16 bytes.
pub fn fast_simple_uuid() -> Result<String, IdError> {
    let id = UuidGenerator::new().next_id()?;
    Ok(UuidGenerator::format_simple_uuid(id))
}

/// Appends fixed-width lowercase hexadecimal digits to a string.
///
/// # Parameters
/// - `output`: Destination string.
/// - `value`: Source value; only the lowest `digits * 4` bits are used.
/// - `digits`: Number of hexadecimal digits to append.
fn push_hex(output: &mut String, value: u128, digits: usize) {
    for index in (0..digits).rev() {
        let nibble = ((value >> (index * 4)) & 0x0f) as usize;
        output.push(char::from(HEX[nibble]));
    }
}
