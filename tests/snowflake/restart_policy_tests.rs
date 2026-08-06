// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for Snowflake restart policy defaults.

use qubit_id::RestartPolicy;

#[test]
fn test_restart_policy_default_is_wait_next_slice() {
    assert_eq!(RestartPolicy::WaitNextSlice, RestartPolicy::default());
}
