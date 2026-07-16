// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Constants shared by Qubit ID generators.

use std::time::Duration;

/// Number of bits used for the host field in Qubit snowflake IDs.
pub const HOST_BITS: u8 = 9;

/// Minimum valid Qubit host identifier.
pub const HOST_MIN: u64 = 0;

/// Maximum valid Qubit host identifier.
pub const HOST_MAX: u64 = (1_u64 << HOST_BITS) - 1;

/// Number of bits used for the mode field in Qubit snowflake IDs.
pub const MODE_BITS: u8 = 1;

/// Number of bits used for the precision field in Qubit snowflake IDs.
pub const PRECISION_BITS: u8 = 1;

/// Timestamp bits used by millisecond precision.
pub const TIMESTAMP_BITS_IN_MILLISECOND: u8 = 41;

/// Sequence bits used by millisecond precision.
pub const SEQUENCE_BITS_IN_MILLISECOND: u8 = 12;

/// Timestamp bits used by second precision.
pub const TIMESTAMP_BITS_IN_SECOND: u8 = 31;

/// Sequence bits used by second precision.
pub const SEQUENCE_BITS_IN_SECOND: u8 = 22;

/// Default Qubit epoch: 2018-12-02T00:00:00Z.
pub const DEFAULT_QUBIT_EPOCH_MILLIS: u64 = 1_543_708_800_000;

/// Default maximum tolerated raw wall-clock rollback.
pub const DEFAULT_MAX_CLOCK_SKEW: Duration = Duration::from_secs(3);
