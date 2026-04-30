/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Common trait for ID generators.

use std::error::Error;
use std::fmt::Display;

/// Generates IDs of type `T`.
///
/// The trait keeps the generated representation generic while still providing a
/// string-producing helper. Numeric generators normally use the default
/// [`Display`] based formatting. Generators with specialized textual forms can
/// override [`IdGenerator::format_id`].
pub trait IdGenerator<T> {
    /// Error returned when generation fails.
    type Error: Error;

    /// Generates the next ID value.
    ///
    /// # Returns
    /// The next generated ID.
    ///
    /// # Errors
    /// Returns `Self::Error` when the generator cannot allocate a unique value,
    /// for example because the clock moved backwards too far, the configured
    /// time range overflowed, or the random source failed.
    fn next_id(&self) -> Result<T, Self::Error>;

    /// Formats an already generated ID.
    ///
    /// # Parameters
    /// - `id`: ID value to format.
    ///
    /// # Returns
    /// String representation of `id`.
    fn format_id(&self, id: &T) -> String
    where
        T: Display,
    {
        id.to_string()
    }

    /// Generates the next ID and formats it as a string.
    ///
    /// # Returns
    /// String representation of the next ID.
    ///
    /// # Errors
    /// Returns the same error as [`IdGenerator::next_id`].
    fn next_string(&self) -> Result<String, Self::Error>
    where
        T: Display,
    {
        self.next_id().map(|id| self.format_id(&id))
    }
}
