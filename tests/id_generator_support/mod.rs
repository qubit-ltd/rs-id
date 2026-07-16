// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared fixtures for the `IdGenerator` trait contract.

mod failing_generator_tests;
mod fixed_generator_tests;
mod opaque_id_tests;

pub(crate) use failing_generator_tests::FailingGenerator;
pub(crate) use fixed_generator_tests::FixedGenerator;
pub(crate) use opaque_id_tests::OpaqueId;
