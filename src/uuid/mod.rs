// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standards-compliant UUID v4 ID generators.

mod internal;
#[allow(clippy::module_inception)]
mod uuid;
mod uuid_v4_generator;

pub use uuid::Uuid;
pub use uuid_v4_generator::UuidV4Generator;
