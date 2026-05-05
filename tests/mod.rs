/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Integration tests for `qubit-id`.

#[path = "error/id_error_tests.rs"]
mod id_error_tests;

#[path = "id_generator/id_generator_tests.rs"]
mod id_generator_tests;

#[path = "snowflake/qubit_snowflake_generator/qubit_snowflake_generator_tests.rs"]
mod qubit_snowflake_generator_tests;

#[path = "snowflake/constants_tests.rs"]
mod snowflake_constants_tests;

#[path = "snowflake/snowflake_builder/snowflake_builder_tests.rs"]
mod snowflake_builder_tests;

#[path = "snowflake/snowflake_generator/snowflake_generator_tests.rs"]
mod snowflake_generator_tests;

#[path = "snowflake/id_mode_tests.rs"]
mod snowflake_id_mode_tests;

#[path = "snowflake/qubit_snowflake_builder_tests.rs"]
mod snowflake_qubit_snowflake_builder_tests;

#[path = "snowflake/sonyflake_generator/sonyflake_generator_tests.rs"]
mod sonyflake_generator_tests;

#[path = "snowflake/time_slice_tests.rs"]
mod snowflake_time_slice_tests;

#[path = "snowflake/timestamp_precision_tests.rs"]
mod snowflake_timestamp_precision_tests;

#[path = "uuid/mica_uuid_like_generator/mica_uuid_like_generator_tests.rs"]
mod mica_uuid_like_generator_tests;
