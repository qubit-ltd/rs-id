// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for `qubit-id`.

mod benchmark_tests;
mod generation_attempt_tests;
mod markdown_tests;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
mod snowflake;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
mod support;
mod try_id_generator_tests;
#[cfg(feature = "uuid")]
mod uuid;
