// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for `qubit-id`.

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
#[cfg(feature = "uuid")]
mod uuid;
