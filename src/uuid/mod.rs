// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standards-compliant UUID v4 ID generators.

mod internal;
mod uuid_v4_generator;
mod uuid_v4_string_generator;

pub use uuid_v4_generator::UuidV4Generator;
pub use uuid_v4_string_generator::UuidV4StringGenerator;
