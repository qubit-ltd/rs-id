// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Common trait for ID generators.

use std::error::Error;

use crate::GenerationOutcome;

/// Generates and formats identifiers.
///
/// Each implementation defines one associated ID representation and its
/// textual format. IDs do not need to implement `Display`.
///
/// Uniqueness is defined by each implementation. Implementations based on a
/// synchronized sequence can provide deterministic guarantees, while random
/// implementations can only provide probabilistic uniqueness.
pub trait IdGenerator {
    /// ID value produced by this generator.
    type Id;
    /// Error returned when generation fails.
    type Error: Error + Send + Sync + 'static;

    /// Performs one generation attempt without sleeping for clock progress or
    /// invoking a sleeper.
    ///
    /// Implementations must return a retry outcome instead of sleeping for
    /// clock progress. A synchronized implementation may still wait briefly
    /// to acquire its internal lock.
    ///
    /// # Returns
    ///
    /// A generated ID or a positive duration after which the caller may retry.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` when the attempt observes a non-retryable
    /// condition, such as an invalid clock value or random-source failure.
    fn try_next_id(&self) -> Result<GenerationOutcome<Self::Id>, Self::Error>;

    /// Generates the next ID value, waiting when generation is retryable.
    ///
    /// Blocking behavior is implementation-specific. Snowflake-family
    /// implementations may wait indefinitely when their wall clock stalls or
    /// their injected sleeper does not cause the wall clock to progress.
    ///
    /// # Returns
    ///
    /// The next generated ID.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` when the generator cannot allocate a unique value,
    /// for example because the clock moved backwards too far, the configured
    /// time range overflowed, a retry sleep failed, or the random source
    /// failed.
    fn next_id(&self) -> Result<Self::Id, Self::Error>;

    /// Formats an already generated ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID value to format.
    ///
    /// # Returns
    ///
    /// String representation of `id`.
    fn format_id(&self, id: &Self::Id) -> String;

    /// Generates the next ID and formats it as a string.
    ///
    /// # Returns
    ///
    /// String representation of the next ID.
    ///
    /// # Errors
    ///
    /// Returns the same error as [`IdGenerator::next_id`].
    #[inline(always)]
    fn next_string(&self) -> Result<String, Self::Error> {
        self.next_id().map(|id| self.format_id(&id))
    }

    /// Performs one non-sleeping generation attempt and formats success.
    ///
    /// # Returns
    ///
    /// A formatted ID or the retry duration returned by
    /// [`IdGenerator::try_next_id`].
    ///
    /// # Errors
    ///
    /// Returns the same error as [`IdGenerator::try_next_id`].
    #[inline(always)]
    fn try_next_string(
        &self,
    ) -> Result<GenerationOutcome<String>, Self::Error> {
        self.try_next_id()
            .map(|outcome| outcome.map(|id| self.format_id(&id)))
    }
}
