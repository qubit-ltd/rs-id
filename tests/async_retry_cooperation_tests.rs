// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg(feature = "qubit-snowflake")]

mod async_retry_cooperation_tests {
    mod internal;

    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::task::Context;
    use std::task::Poll;
    use std::task::Wake;
    use std::task::Waker;
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use internal::AdvancingFuseTimer;
    use internal::REGISTRATION_LIMIT;
    use internal::manual_wall_clock;
    use qubit_clock::FixedWallClock;
    use qubit_clock::ManualMonotonicClock;
    use qubit_clock::TimeError;
    use qubit_id::AsyncIdGenerator;
    use qubit_id::Id;
    use qubit_id::IdGenerationError;
    use qubit_id::RestartPolicy;
    use qubit_id::SnowflakeGenerator;
    use qubit_id::TimestampPrecision;

    #[derive(Default)]
    struct CountWake(AtomicUsize);

    impl Wake for CountWake {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn make_generator(timer: Arc<AdvancingFuseTimer>) -> SnowflakeGenerator {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let frozen_now = epoch + Duration::from_millis(9) + Duration::from_nanos(999_999);
        SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(Arc::new(FixedWallClock::new(frozen_now)))
            .timer(timer)
            .build()
            .expect("generator configuration should be valid")
    }

    fn poll_once(
        future: Pin<&mut dyn Future<Output = Result<Id, IdGenerationError>>>,
    ) -> (Poll<Result<Id, IdGenerationError>>, usize) {
        let wake = Arc::new(CountWake::default());
        let waker = Waker::from(Arc::clone(&wake));
        let mut context = Context::from_waker(&waker);
        let result = future.poll(&mut context);
        (result, wake.0.load(Ordering::SeqCst))
    }

    #[test]
    fn test_async_retry_poll_cooperates_for_concrete_and_dyn_futures() {
        let clock = ManualMonotonicClock::new_shared();
        let timer = Arc::new(AdvancingFuseTimer::new(clock, REGISTRATION_LIMIT, false));
        let generator = make_generator(Arc::clone(&timer));
        let mut concrete = Box::pin(generator.generate_async());
        let (concrete_result, concrete_wakes) = poll_once(concrete.as_mut());
        let concrete_registrations = timer.registrations();
        drop(concrete);

        let clock = ManualMonotonicClock::new_shared();
        let timer = Arc::new(AdvancingFuseTimer::new(clock, REGISTRATION_LIMIT, false));
        let generator = make_generator(Arc::clone(&timer));
        let dyn_generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(generator);
        let mut dynamic = Box::pin(dyn_generator.generate_async());
        let (dynamic_result, dynamic_wakes) = poll_once(dynamic.as_mut());
        let dynamic_registrations = timer.registrations();
        drop(dynamic);

        assert!(matches!(concrete_result, Poll::Pending));
        assert_eq!(concrete_registrations, 1, "one outer poll may register one wait");
        assert_eq!(concrete_wakes, 1, "a pending retry must schedule another poll");
        assert!(matches!(dynamic_result, Poll::Pending));
        assert_eq!(dynamic_registrations, 1, "one outer poll may register one wait");
        assert_eq!(dynamic_wakes, 1, "a pending retry must schedule another poll");
    }

    #[test]
    fn test_async_retry_registration_error_is_preserved() {
        let clock = ManualMonotonicClock::new_shared();
        let timer = Arc::new(AdvancingFuseTimer::new(clock, 1, false));
        let generator = make_generator(Arc::clone(&timer));
        let mut concrete = Box::pin(generator.generate_async());
        let (concrete_result, _) = poll_once(concrete.as_mut());

        assert!(matches!(
            concrete_result,
            Poll::Ready(Err(IdGenerationError::WaitFailed {
                source: TimeError::InstantOverflow,
            }))
        ));
        assert_eq!(timer.registrations(), 1);
    }

    #[test]
    fn test_async_retry_completion_error_is_preserved_through_dyn_future() {
        let clock = ManualMonotonicClock::new_shared();
        let timer = Arc::new(AdvancingFuseTimer::new(clock, REGISTRATION_LIMIT, true));
        let generator = make_generator(Arc::clone(&timer));
        let dyn_generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(generator);
        let mut dynamic = Box::pin(dyn_generator.generate_async());
        let (result, _) = poll_once(dynamic.as_mut());

        assert!(matches!(
            result,
            Poll::Ready(Err(IdGenerationError::WaitFailed {
                source: TimeError::InstantOverflow,
            }))
        ));
        assert_eq!(timer.registrations(), 1);
    }

    #[test]
    fn test_cancelled_retry_future_releases_generator_for_later_allocation() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let clock = ManualMonotonicClock::new_shared();
        let wall_clock = manual_wall_clock(
            epoch + Duration::from_millis(9) + Duration::from_nanos(999_999),
            Arc::clone(&clock),
        );
        let timer = Arc::new(AdvancingFuseTimer::new(clock, REGISTRATION_LIMIT, false));
        let generator = SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(wall_clock.clone())
            .timer(timer)
            .build()
            .expect("generator configuration should be valid");

        let mut cancelled = Box::pin(generator.generate_async());
        let (result, _) = poll_once(cancelled.as_mut());
        assert!(matches!(result, Poll::Pending));
        drop(cancelled);

        wall_clock.reanchor(epoch + Duration::from_millis(10));
        let mut retry = Box::pin(generator.generate_async());
        assert!(matches!(poll_once(retry.as_mut()).0, Poll::Ready(Ok(_))));
    }
}
