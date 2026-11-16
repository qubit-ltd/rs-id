// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `AsyncIdGenerator` trait contract.

mod id_generator_support;

use std::convert::Infallible;
use std::io::Error as IoError;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_id::AsyncIdGenerator;
use qubit_id::IdGenerationFuture;
use tokio::runtime::Builder;
use tokio::spawn;

use self::id_generator_support::counter_generator_tests::CounterGenerator;
use self::id_generator_support::counter_generator_tests::IoCounterGenerator;

struct AsyncOnlyGenerator;

impl AsyncIdGenerator<u64, Infallible> for AsyncOnlyGenerator {
    fn generate_async(&self) -> IdGenerationFuture<'_, u64, Infallible> {
        Box::pin(async { Ok(42) })
    }
}

#[test]
fn test_async_id_generator_can_be_used_without_blocking_capability() {
    let generator: Arc<dyn AsyncIdGenerator<u64, Infallible>> = Arc::new(AsyncOnlyGenerator);
    let mut future = generator.generate_async();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Ready(Ok(42))));
}

#[test]
fn test_async_id_generator_is_object_safe_for_one_output_type() {
    let generator: Arc<dyn AsyncIdGenerator<u64>> = Arc::new(CounterGenerator::default());
    let mut future = generator.generate_async();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    match future.as_mut().poll(&mut context) {
        Poll::Ready(Ok(id)) => assert_eq!(id, 1),
        other => panic!("expected a ready identifier, got {other:?}"),
    }
}

#[test]
fn test_async_id_generator_supports_custom_error_type() {
    let generator: Arc<dyn AsyncIdGenerator<u64, IoError>> = Arc::new(IoCounterGenerator::default());
    let mut future = generator.generate_async();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    match future.as_mut().poll(&mut context) {
        Poll::Ready(Ok(id)) => assert_eq!(id, 1),
        other => panic!("expected a ready identifier, got {other:?}"),
    }
}

#[test]
fn test_async_id_generator_send_error_can_run_in_spawned_task() {
    let generator: Arc<dyn AsyncIdGenerator<u64, IoError>> = Arc::new(IoCounterGenerator::default());
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build");

    let value = runtime
        .block_on(async move { spawn(async move { generator.generate_async().await }).await })
        .expect("spawned generation should complete")
        .expect("generation should succeed");

    assert_eq!(value, 1);
}

#[test]
fn test_async_id_generator_trait_object_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync + ?Sized>() {}

    assert_send_sync::<dyn AsyncIdGenerator<u64>>();
}
