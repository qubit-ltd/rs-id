// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for numeric UUID v4 generation.

use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use qubit_id::{
    IdGenerator,
    UuidV4Generator,
};
use uuid::{
    Uuid,
    Variant,
};

#[test]
fn test_uuid_v4_generator_returns_standard_uuid_bits() {
    let generator = UuidV4Generator::new();

    let value = generator.generate().expect("UUID should generate");
    let uuid = Uuid::from_u128(value);

    assert_eq!(uuid.get_version_num(), 4);
    assert_eq!(uuid.get_variant(), Variant::RFC4122);
}

#[test]
fn test_uuid_v4_generator_supports_sync_trait_object() {
    let generator: Arc<dyn IdGenerator<u128>> =
        Arc::new(UuidV4Generator::new());

    assert!(generator.generate().is_ok());
}

#[test]
fn test_uuid_v4_generator_is_unique_across_concurrent_sample() {
    const WORKERS: usize = 8;
    const IDS_PER_WORKER: usize = 1_000;

    let generator: Arc<dyn IdGenerator<u128>> =
        Arc::new(UuidV4Generator::new());
    let workers = (0..WORKERS)
        .map(|_| {
            let generator = Arc::clone(&generator);
            thread::spawn(move || {
                (0..IDS_PER_WORKER)
                    .map(|_| {
                        generator.generate().expect("UUID should generate")
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let generated = workers
        .into_iter()
        .flat_map(|worker| worker.join().expect("worker should finish"))
        .collect::<HashSet<_>>();

    assert_eq!(generated.len(), WORKERS * IDS_PER_WORKER);
}
