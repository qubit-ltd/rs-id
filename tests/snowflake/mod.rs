// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#[cfg(feature = "qubit-snowflake")]
mod constants_tests;
#[cfg(feature = "qubit-snowflake")]
mod id_mode_tests;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_generator_builder_tests;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_generator_tests;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_layout_tests;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_parts_tests;
mod restart_policy_tests;
#[cfg(feature = "classic-snowflake")]
mod snowflake_generator_builder_tests;
#[cfg(feature = "classic-snowflake")]
mod snowflake_generator_tests;
#[cfg(feature = "classic-snowflake")]
mod snowflake_layout_tests;
#[cfg(feature = "classic-snowflake")]
mod snowflake_parts_tests;
#[cfg(feature = "sonyflake")]
mod sonyflake_generator_builder_tests;
#[cfg(feature = "sonyflake")]
mod sonyflake_generator_tests;
#[cfg(feature = "sonyflake")]
mod sonyflake_layout_tests;
#[cfg(feature = "sonyflake")]
mod sonyflake_parts_tests;
#[cfg(feature = "qubit-snowflake")]
mod timestamp_precision_tests;
