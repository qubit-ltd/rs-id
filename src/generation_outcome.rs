// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Result of one ID generation attempt that does not invoke a sleeper.

use std::time::Duration;

/// Result of one ID generation attempt that does not invoke a sleeper.
///
/// `RetryAfter` is a scheduling instruction, not an error. The caller can
/// retry after the returned positive duration or choose its own cancellation
/// and backoff policy. `T` is the generated ID representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "generation outcomes must be handled"]
pub enum GenerationOutcome<T> {
    /// An ID was generated successfully.
    Generated(
        /// Generated ID value.
        T,
    ),
    /// Generation can be retried after the specified positive duration.
    RetryAfter(
        /// Positive duration recommended before the next attempt.
        Duration,
    ),
}

impl<T> GenerationOutcome<T> {
    /// Transforms a generated value while preserving a retry instruction.
    ///
    /// `U` is the transformed ID representation and `F` is the one-shot
    /// transformation applied only to [`GenerationOutcome::Generated`].
    ///
    /// # Arguments
    ///
    /// * `transform` - Function applied to a generated value.
    ///
    /// # Returns
    ///
    /// A generated transformed value, or the original retry instruction.
    #[inline]
    pub fn map<U, F>(self, transform: F) -> GenerationOutcome<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Generated(value) => {
                GenerationOutcome::Generated(transform(value))
            }
            Self::RetryAfter(duration) => {
                GenerationOutcome::RetryAfter(duration)
            }
        }
    }
}
