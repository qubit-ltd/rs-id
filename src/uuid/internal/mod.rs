// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared UUID generation internals.

mod generate_uuid_v4;

pub(super) use generate_uuid_v4::generate_uuid_v4;
