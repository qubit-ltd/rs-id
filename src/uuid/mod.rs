/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! UUID-like ID generators and formatting helpers.

mod mica_uuid_like_generator;

pub use mica_uuid_like_generator::{
    MicaUuidLikeGenerator,
    fast_simple_uuid_like,
    fast_uuid_like,
};
