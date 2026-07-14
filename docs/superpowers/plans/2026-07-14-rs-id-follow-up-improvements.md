# rs-id Follow-up Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce duplicated Snowflake allocation logic, complete configuration introspection, and clarify the remaining Qubit documentation without changing allocation semantics.

**Architecture:** Keep `parking_lot::Mutex<Option<TimeSlice>>` as the synchronization backend. Move the common startup-fence, rollback detection, time-slice advance, sequence increment, and exhaustion decisions into a private lock-independent transition function; each generator retains its own clock, wait, rollback-policy, and ID-composition behavior.

**Tech Stack:** Rust 2024, `parking_lot`, integration tests under `tests/`, Cargo test and Clippy.

## Global Constraints

- Work directly in the current `rs-id` checkout on branch `dev-starfish`.
- Do not introduce `qubit-fast-cas` or change synchronization semantics.
- Preserve the startup fence and all existing uniqueness, rollback, and blocking contracts.
- Do not create commits; the user did not authorize `git add` or `git commit`.
- Keep all tests under `tests/`; do not add inline `#[cfg(test)]` modules.

---

### Task 1: Complete generator configuration accessors

**Files:**
- Modify: `tests/snowflake/qubit_snowflake_generator/qubit_snowflake_generator_tests.rs`
- Modify: `tests/snowflake/sonyflake_generator/sonyflake_generator_tests.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`

**Interfaces:**
- Produces: `QubitSnowflakeGenerator::max_skew_millis(&self) -> u64`
- Produces: `SonyflakeGenerator::machine_id(&self) -> u64`
- Produces: `SonyflakeGenerator::start_time(&self) -> SystemTime`
- Produces: `SonyflakeGenerator::time_unit(&self) -> Duration`

- [x] **Step 1: Write failing accessor tests**

Add a Qubit test that constructs a generator with a known epoch and skew, then asserts `layout()`, `epoch()`, and `max_skew_millis()`. Add a Sonyflake test that constructs a generator with known configuration, then asserts `machine_id()`, `start_time()`, `time_unit()`, and the existing bit-length accessors.

```rust
#[test]
fn test_qubit_snowflake_generator_accessors_return_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
        epoch,
        37,
        move || epoch + Duration::from_millis(100),
    )
    .expect("configuration should be valid");
    let expected_layout = QubitSnowflakeLayout::new(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
    )
    .expect("layout should be valid");

    assert_eq!(generator.layout(), &expected_layout);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.max_skew_millis(), 37);
}

#[test]
fn test_sonyflake_generator_accessors_return_configuration() {
    let start_time = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time_unit = Duration::from_millis(5);
    let generator = SonyflakeGenerator::with_clock(
        17,
        7,
        5,
        time_unit,
        start_time,
        move || start_time + Duration::from_millis(100),
    )
    .expect("configuration should be valid");

    assert_eq!(generator.machine_id(), 17);
    assert_eq!(generator.start_time(), start_time);
    assert_eq!(generator.time_unit(), time_unit);
    assert_eq!(generator.bits_time(), 51);
    assert_eq!(generator.bits_sequence(), 7);
    assert_eq!(generator.bits_machine(), 5);
}
```

- [x] **Step 2: Run the tests and verify RED**

Run: `cargo test --test mod accessors_return_configuration`

Expected: compilation fails with `E0599` because the four new accessors do not exist.

- [x] **Step 3: Implement the minimal accessors**

Add documented `const fn` getters returning the existing immutable fields. Do not add setters or new configuration types.

```rust
pub const fn max_skew_millis(&self) -> u64 {
    self.max_skew_millis
}

pub const fn machine_id(&self) -> u64 {
    self.machine_id
}

pub const fn start_time(&self) -> SystemTime {
    self.start_time
}

pub const fn time_unit(&self) -> Duration {
    self.time_unit
}
```

- [x] **Step 4: Run the focused tests and verify GREEN**

Run: `cargo test --test mod accessors_return_configuration`

Expected: both accessor tests pass.

### Task 2: Extract the shared allocation transition under green tests

**Files:**
- Modify: `src/snowflake/time_slice.rs`
- Create: `src/snowflake/time_slice_reservation.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`

**Interfaces:**
- Produces: private `TimeSliceReservation` with `Allocated(TimeSlice)`, `WaitForNext(u64)`, and `ClockMovedBackwards { last_timestamp, current_timestamp }` variants.
- Produces: private `reserve_next(&mut Option<TimeSlice>, u64, u64) -> TimeSliceReservation`.

- [x] **Step 1: Establish the green characterization baseline**

Run: `cargo test --test mod snowflake_generator && cargo test --test mod qubit_snowflake_generator && cargo test --test mod sonyflake_generator`

Expected: all existing generator behavior tests pass before refactoring.

- [x] **Step 2: Implement the private transition**

The transition must:

- install the startup fence at `max_sequence` and return `WaitForNext` for `None`;
- return `ClockMovedBackwards` without mutation when the stored timestamp exceeds the current timestamp;
- reset to sequence zero and return `Allocated` when time advances;
- return `WaitForNext` without mutation when the sequence is exhausted;
- increment the sequence and return `Allocated` in the current time slice.

```rust
pub(crate) enum TimeSliceReservation {
    Allocated(TimeSlice),
    WaitForNext(u64),
    ClockMovedBackwards {
        last_timestamp: u64,
        current_timestamp: u64,
    },
}

pub(crate) fn reserve_next(
    state: &mut Option<TimeSlice>,
    current_timestamp: u64,
    max_sequence: u64,
) -> TimeSliceReservation {
    let Some(time_slice) = state.as_mut() else {
        *state = Some(TimeSlice::with_sequence(
            current_timestamp,
            max_sequence,
        ));
        return TimeSliceReservation::WaitForNext(current_timestamp);
    };
    if time_slice.timestamp > current_timestamp {
        return TimeSliceReservation::ClockMovedBackwards {
            last_timestamp: time_slice.timestamp,
            current_timestamp,
        };
    }
    if current_timestamp > time_slice.timestamp {
        *time_slice = TimeSlice::new(current_timestamp);
        return TimeSliceReservation::Allocated(*time_slice);
    }
    if time_slice.sequence == max_sequence {
        return TimeSliceReservation::WaitForNext(time_slice.timestamp);
    }
    time_slice.sequence += 1;
    TimeSliceReservation::Allocated(*time_slice)
}
```

- [x] **Step 3: Refactor all three generators to consume the transition**

Compute the clock timestamp while holding the existing mutex, call `reserve_next`, then let the guard leave scope before composing IDs, sleeping, or waiting. Preserve Qubit's bounded rollback wait and the immediate rollback errors in Classic and Sonyflake.

Each generator must use a scoped reservation followed by its own match:

```rust
let reservation = {
    let mut state = self.state.lock();
    let timestamp = self.current_timestamp()?;
    reserve_next(&mut state, timestamp, self.max_sequence())
};
```

Classic and Sonyflake map `Allocated` to their existing `compose` calls, map
`WaitForNext` to their existing wait helpers, and map `ClockMovedBackwards` to
their existing immediate `IdError`. Qubit maps the same variants to
`layout.compose`, `wait_for_next_timestamp`, and its existing bounded-skew
sleep/error branch. No clock read, sleep, wait, or ID composition may occur
while the mutex guard is alive except the clock read used to create the
reservation.

- [x] **Step 4: Run the characterization tests after refactoring**

Run: `cargo test --test mod snowflake_generator && cargo test --test mod qubit_snowflake_generator && cargo test --test mod sonyflake_generator`

Expected: all generator tests remain green, including concurrent overflow, restart fence, rollback, and panic recovery cases.

### Task 3: Correct and clarify documentation

**Files:**
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `README.md`
- Modify: `README.zh_CN.md`

**Interfaces:**
- No API changes.

- [x] **Step 1: Correct stale Qubit terminology**

Replace “builder validation errors” in `generate_at` documentation with explicit `TimestampOverflow` and `SequenceOverflow` layout range errors.

```rust
/// Returns [`IdError::TimeBeforeEpoch`] if `time` is before the configured
/// epoch. Returns [`IdError::TimestampOverflow`] or
/// [`IdError::SequenceOverflow`] when the computed timestamp or provided
/// sequence does not fit the layout.
```

- [x] **Step 2: Clarify default startup latency**

State in Qubit Rustdoc and both READMEs that the default second-precision generator can wait nearly one second on its first generation call because the startup fence skips the observed time slice. Do not add an opt-out policy.

Use equivalent wording in both languages:

```text
With the default second precision, the startup fence means the first generation
call can wait nearly one second.
```

```text
使用默认的秒精度时，启动栅栏会使首次生成调用最多等待接近一秒。
```

- [x] **Step 3: Verify documentation and formatting**

Run: `cargo +nightly-2026-06-05 fmt -- --check --config-path .rs-ci/rustfmt.toml && cargo test --doc`

Expected: formatting is clean and all doctests pass.

### Task 4: Full verification

**Files:**
- Verify all modified files.

**Interfaces:**
- No new interfaces.

- [x] **Step 1: Run the complete test suite**

Run: `cargo test --all-targets`

Expected: all unit, integration, and benchmark smoke tests pass.

- [x] **Step 2: Run Clippy with warnings denied**

Run: `cargo clippy --all-targets -- -D warnings`

Expected: exits successfully with no warnings.

- [x] **Step 3: Inspect the final diff and worktree state**

Run: `git --no-pager diff --check && git status --short && git --no-pager diff`

Expected: no whitespace errors; only the plan, intended Rust sources, tests, and READMEs are modified.
