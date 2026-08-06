// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared fixtures for the synchronous and asynchronous generator contracts.

mod counter_generator_tests;

#[allow(unused_imports)]
pub(crate) use counter_generator_tests::{
    CounterGenerator,
    IoCounterGenerator,
};
