/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Integration tests for `qubit-id`.

#[path = "error/id_error_tests.rs"]
mod id_error_tests;

#[path = "id_generator/id_generator_tests.rs"]
mod id_generator_tests;

#[path = "qubit_snowflake_generator/qubit_snowflake_generator_tests.rs"]
mod qubit_snowflake_generator_tests;

#[path = "snowflake_builder/snowflake_builder_tests.rs"]
mod snowflake_builder_tests;

#[path = "snowflake_generator/snowflake_generator_tests.rs"]
mod snowflake_generator_tests;

#[path = "sonyflake_generator/sonyflake_generator_tests.rs"]
mod sonyflake_generator_tests;

#[path = "mica_uuid_like_generator/mica_uuid_like_generator_tests.rs"]
mod mica_uuid_like_generator_tests;
