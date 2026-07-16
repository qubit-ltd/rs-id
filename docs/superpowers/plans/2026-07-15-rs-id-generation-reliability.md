# rs-id Generation Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 rs-id 增加显式重启策略、非阻塞生成接口和 rs-clock 可测试时钟，并修复量化前时钟回拨判断、错误链、文档及 Rust 代码组织问题。

**Architecture:** 公共层使用关联类型 `IdGenerator` 和 `GenerationOutcome` 区分成功、可恢复等待与错误。三种 Snowflake 通过一个 mutex 保护共享的原始时钟高水位、启动门槛和时间片序列状态；`try_next_id()` 只执行一次状态转换，`next_id()` 使用注入的 `BlockingSleeper` 适配为阻塞调用。

**Tech Stack:** Rust 1.94、edition 2024、`parking_lot`、`getrandom` 0.4、`qubit-clock` 0.9、`thiserror` 2.0、Cargo 集成测试与项目 CI 脚本。

## Global Constraints

- 设计依据是 `docs/superpowers/specs/2026-07-15-rs-id-generation-reliability-design.md`。
- Qubit、Classic Snowflake 和 Sonyflake 的现有数值位布局、默认 epoch/start time、节点字段及序列字段不得改变。
- `RestartPolicy::Immediate` 是默认策略；`WaitNextSlice` 从第一次生成调用观察到的时间片起等待。
- 不增加持久化状态、符号位、版本位、异步 API 或跨进程协调。
- 本轮保留共享 mutex，不实施原子状态机或批量序列租约。
- `try_next_id()` 和 `try_next_string()` 不得调用 sleeper 或发生真实休眠。
- 所有时间测试使用确定性时钟；真实时间只允许作为防止测试永久挂起的超时保护。
- 测试必须位于 `tests/`，路径与源码相对应，不在 `src/` 内嵌测试。
- 每个新增或移动的 Rust 类型单独放置，并为公共及私有函数、字段补全英文 rustdoc。
- 未经用户明确授权，不执行 `git add`、`git commit` 或 `git push`。每个任务末尾的提交命令仅是授权后的检查点，不默认执行。
- 修改源码必须使用 `apply_patch`；格式化和 Cargo 自动更新 `Cargo.lock` 属于机械变更。

---

## File Map

### Public API

- `src/generation_outcome.rs`：一次非阻塞生成尝试的公共结果。
- `src/id_generator.rs`：关联 ID 类型的生成器 trait。
- `src/id_error.rs`：带底层 source 和原始时间上下文的公共错误。
- `src/snowflake/restart_policy.rs`：显式重启策略。
- `src/snowflake/snowflake_generator_builder.rs`：Classic Snowflake builder。
- `src/lib.rs`、`src/snowflake/mod.rs`：模块声明与公共重导出。

### Shared Snowflake Internals

- `src/snowflake/internal/mod.rs`：仅声明并重导出私有实现单元。
- `src/snowflake/internal/clock_observation.rs`：原始 elapsed time、量化时间片和下一边界等待时长。
- `src/snowflake/internal/generation_state.rs`：原始时钟高水位、启动门槛和序列分配状态机。
- `src/snowflake/internal/restart_fence.rs`：`Immediate`/`WaitNextSlice` 的启动状态。
- `src/snowflake/internal/time_slice.rs`：已分配的时间片与序列。
- `src/snowflake/internal/block_until_generated.rs`：使用 `BlockingSleeper` 的同步适配循环。
- `src/snowflake/internal/clock_defaults.rs`：标准墙上时钟和标准阻塞 sleeper 工厂。
- 删除 `src/snowflake/time_slice.rs` 和 `src/snowflake/time_slice_reservation.rs`。

### Tests and Test Support

- `tests/support/manual_time.rs`：共享的手动单调时钟、墙上时钟和 sleeper fixture。
- `tests/support/failing_blocking_sleeper.rs`：稳定返回 `TimeError` 的 sleeper。
- `tests/support/fixed_generator.rs`、`tests/support/failing_generator.rs`、`tests/support/opaque_id.rs`：trait 契约测试类型。
- 将根源码的测试平铺为 `tests/id_error_tests.rs`、`tests/id_generator_tests.rs` 和 `tests/generation_outcome_tests.rs`。
- 将生成器测试平铺为 `tests/snowflake/*_generator_tests.rs`，将 Mica 测试平铺为 `tests/uuid/mica_uuid_like_generator_tests.rs`。

### Documentation and Benchmark

- `README.md`、`README.zh_CN.md`：同步公共 API、重复条件、阻塞边界及 64 位布局限制。
- `benches/qubit_snowflake_throughput/throughput_sample.rs`、`throughput_summary.rs`、`startup_latency_summary.rs`：拆分 benchmark 类型，不改变测量方法。
- `docs/benchmarks/qubit-snowflake-throughput.md`：记录重构后的实测结果和 mutex 后续议题。

---

### Task 1: Introduce the non-blocking public contract

**Files:**
- Create: `src/generation_outcome.rs`
- Create: `src/snowflake/restart_policy.rs`
- Modify: `src/id_generator.rs`
- Modify: `src/lib.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`
- Modify: `src/uuid/mica_uuid_like_generator.rs`
- Create: `tests/generation_outcome_tests.rs`
- Create: `tests/snowflake/restart_policy_tests.rs`
- Move: `tests/id_generator/id_generator_tests.rs` to `tests/id_generator_tests.rs`
- Move: `tests/uuid/mica_uuid_like_generator/mica_uuid_like_generator_tests.rs` to `tests/uuid/mica_uuid_like_generator_tests.rs`
- Create: `tests/support/opaque_id.rs`
- Create: `tests/support/fixed_generator.rs`
- Create: `tests/support/failing_generator.rs`
- Create: `tests/support/mod.rs`
- Modify: `tests/mod.rs`
- Modify: `tests/snowflake/mod.rs`
- Modify: `tests/uuid/mod.rs`
- Delete: `tests/id_generator/mod.rs`
- Delete: `tests/uuid/mica_uuid_like_generator/mod.rs`

**Interfaces:**
- Produces: `GenerationOutcome<T>`, `GenerationOutcome::map`, `RestartPolicy`, and the associated-type `IdGenerator` methods from the approved spec.
- Preserves: current quantized rollback behavior until Task 2 supplies raw clock observations.

- [ ] **Step 1: Add failing public-contract tests**

Add these cases before changing production code:

```rust
#[test]
fn test_generation_outcome_map_transforms_generated_value() {
    let outcome = GenerationOutcome::Generated(21_u64).map(|value| value * 2);
    assert_eq!(outcome, GenerationOutcome::Generated(42));
}

#[test]
fn test_generation_outcome_map_preserves_retry_after() {
    let duration = Duration::from_millis(25);
    let outcome = GenerationOutcome::<u64>::RetryAfter(duration)
        .map(|value| value.to_string());
    assert_eq!(outcome, GenerationOutcome::RetryAfter(duration));
}

#[test]
fn test_restart_policy_default_is_immediate() {
    assert_eq!(RestartPolicy::Immediate, RestartPolicy::default());
}

#[test]
fn test_id_generator_formats_id_without_display() {
    let generator = FixedGenerator::new(42);
    assert_eq!(
        generator.next_string().expect("fixed generation should succeed"),
        "opaque:42"
    );
    assert_eq!(
        generator
            .try_next_string()
            .expect("fixed generation should succeed"),
        GenerationOutcome::Generated("opaque:42".to_owned())
    );
}

#[test]
fn test_id_generator_try_next_string_propagates_error() {
    let error = FailingGenerator
        .try_next_string()
        .expect_err("failing generation should return its error");
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
}
```

`OpaqueId` intentionally has no `Display` implementation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpaqueId {
    /// Numeric payload used by the fixture formatter.
    pub(crate) value: u64,
}
```

The two generator fixtures are:

```rust
pub(crate) struct FixedGenerator {
    /// Numeric payload returned by every attempt.
    value: u64,
}

impl FixedGenerator {
    /// Creates a fixed generator for `value`.
    pub(crate) const fn new(value: u64) -> Self {
        Self { value }
    }
}

impl IdGenerator for FixedGenerator {
    type Id = OpaqueId;
    type Error = Infallible;

    fn try_next_id(
        &self,
    ) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        Ok(GenerationOutcome::Generated(OpaqueId {
            value: self.value,
        }))
    }

    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        Ok(OpaqueId { value: self.value })
    }

    fn format_id(&self, id: &Self::Id) -> String {
        format!("opaque:{}", id.value)
    }
}
```

```rust
pub(crate) struct FailingGenerator;

impl IdGenerator for FailingGenerator {
    type Id = OpaqueId;
    type Error = std::io::Error;

    fn try_next_id(
        &self,
    ) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        Err(std::io::Error::other("fixture generation failed"))
    }

    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        Err(std::io::Error::other("fixture generation failed"))
    }

    fn format_id(&self, id: &Self::Id) -> String {
        format!("opaque:{}", id.value)
    }
}
```

Also add a built-in Qubit test using millisecond precision and a fixed clock:
the first 4,096 attempts must be `Generated`, and attempt 4,097 must be
`RetryAfter(Duration::from_millis(1))` without advancing or sleeping.

- [ ] **Step 2: Run the contract tests and observe the expected red state**

Run:

```bash
cargo test --test mod generation_outcome_tests
cargo test --test mod id_generator_tests
cargo test --test mod restart_policy_tests
```

Expected: compilation fails because `GenerationOutcome`, `RestartPolicy`, associated `Id`, `try_next_id`, and `try_next_string` do not exist.

- [ ] **Step 3: Implement `GenerationOutcome` and `RestartPolicy`**

Create the concrete public types:

```rust
use std::time::Duration;

/// Result of one non-blocking ID generation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "generation outcomes must be handled"]
pub enum GenerationOutcome<T> {
    /// An ID was generated successfully.
    Generated(T),
    /// Generation can be retried after the specified non-zero duration.
    RetryAfter(Duration),
}

impl<T> GenerationOutcome<T> {
    /// Transforms a generated value while preserving a retry instruction.
    pub fn map<U, F>(self, transform: F) -> GenerationOutcome<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Generated(value) => {
                GenerationOutcome::Generated(transform(value))
            }
            Self::RetryAfter(duration) => {
                GenerationOutcome::RetryAfter(duration)
            }
        }
    }
}
```

```rust
/// Determines when a fresh Snowflake generator may allocate its first ID.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestartPolicy {
    /// Allocates immediately and may repeat IDs after same-slice state loss.
    #[default]
    Immediate,
    /// Waits until the slice after the first observed slice.
    WaitNextSlice,
}
```

- [ ] **Step 4: Replace the generic trait with its associated-type contract**

Replace `IdGenerator<T>` with:

```rust
use std::error::Error;

use crate::GenerationOutcome;

/// Generates and formats identifiers.
pub trait IdGenerator {
    /// ID value produced by this generator.
    type Id;
    /// Error returned when generation fails.
    type Error: Error + Send + Sync + 'static;

    /// Performs one generation attempt without sleeping.
    fn try_next_id(
        &self,
    ) -> Result<GenerationOutcome<Self::Id>, Self::Error>;

    /// Generates the next ID, waiting when the implementation can recover.
    fn next_id(&self) -> Result<Self::Id, Self::Error>;

    /// Formats an already generated ID.
    fn format_id(&self, id: &Self::Id) -> String;

    /// Generates and formats the next ID.
    fn next_string(&self) -> Result<String, Self::Error> {
        self.next_id().map(|id| self.format_id(&id))
    }

    /// Performs one non-blocking generation attempt and formats success.
    fn try_next_string(
        &self,
    ) -> Result<GenerationOutcome<String>, Self::Error> {
        self.try_next_id()
            .map(|outcome| outcome.map(|id| self.format_id(&id)))
    }
}
```

- [ ] **Step 5: Give every existing generator a one-attempt implementation**

For each Snowflake generator, move one iteration of the current reservation loop into `try_next_id()`. Map `Allocated` to `Generated`, sequence exhaustion to a positive `RetryAfter`, tolerated Qubit rollback to `RetryAfter`, and fatal rollback to `IdError`. Make `next_id()` loop over this result:

```rust
fn next_id(&self) -> Result<Self::Id, Self::Error> {
    loop {
        match self.try_next_id()? {
            GenerationOutcome::Generated(id) => return Ok(id),
            GenerationOutcome::RetryAfter(duration) => {
                std::thread::sleep(duration);
            }
        }
    }
}

fn format_id(&self, id: &Self::Id) -> String {
    id.to_string()
}
```

At this checkpoint, Classic uses `Duration::from_millis(1)`, Qubit uses the existing precision wait duration, and Sonyflake uses `self.time_unit` for sequence exhaustion. These fixed waits are replaced by exact boundary durations in Task 2.

For Mica, share a private non-blocking random read rather than adding an impossible retry path:

```rust
fn generate_id() -> Result<u128, IdError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| IdError::RandomSourceUnavailable)?;
    Ok(u128::from_be_bytes(bytes))
}

impl IdGenerator for MicaUuidLikeGenerator {
    type Id = u128;
    type Error = IdError;

    fn try_next_id(
        &self,
    ) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        generate_id().map(GenerationOutcome::Generated)
    }

    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        generate_id()
    }

    fn format_id(&self, id: &Self::Id) -> String {
        Self::format_uuid_like(*id)
    }
}
```

- [ ] **Step 6: Update exports, test modules, and moved tests**

Export both new public types from `src/lib.rs`, export `RestartPolicy` from `src/snowflake/mod.rs`, declare the new test modules directly in `tests/mod.rs`, and replace nested UUID/generator test module declarations with flat `*_tests` declarations. Preserve all existing test bodies while updating trait syntax and imports.

- [ ] **Step 7: Run the focused and full test suites**

Run:

```bash
cargo test --test mod generation_outcome_tests
cargo test --test mod id_generator_tests
cargo test --test mod mica_uuid_like_generator_tests
cargo test
```

Expected: all tests pass; `try_next_id()` is public and non-blocking, while raw rollback behavior is still covered by Task 2.

- [ ] **Step 8: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add src tests
git commit -m "feat(id): 增加非阻塞生成契约"
```

---

### Task 2: Detect raw clock rollback before quantization

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/id_error.rs`
- Create: `src/snowflake/internal/mod.rs`
- Create: `src/snowflake/internal/clock_observation.rs`
- Create: `src/snowflake/internal/generation_state.rs`
- Create: `src/snowflake/internal/restart_fence.rs`
- Move: `src/snowflake/time_slice.rs` to `src/snowflake/internal/time_slice.rs`
- Delete: `src/snowflake/time_slice_reservation.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/snowflake/constants.rs`
- Modify: `src/snowflake/timestamp_precision.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`
- Modify: `src/uuid/mica_uuid_like_generator.rs`
- Move: `tests/error/id_error_tests.rs` to `tests/id_error_tests.rs`
- Modify: `tests/mod.rs`
- Modify: `tests/snowflake/qubit_snowflake_generator/qubit_snowflake_generator_tests.rs`
- Modify: `tests/snowflake/snowflake_generator/snowflake_generator_tests.rs`
- Modify: `tests/snowflake/sonyflake_generator/sonyflake_generator_tests.rs`
- Modify: `tests/snowflake/constants_tests.rs`
- Modify: `tests/snowflake/timestamp_precision_tests.rs`
- Delete: `tests/error/mod.rs`
- Delete: `tests/snowflake/time_slice_tests.rs`

**Interfaces:**
- Consumes: `GenerationOutcome<T>` and `RestartPolicy` from Task 1.
- Produces: `ClockObservation::from_time`, `GenerationState::reserve`, exact `RetryAfter`, and the final contextual `IdError` shape.

- [ ] **Step 1: Add the required failing Qubit regression test before the fix**

Use the existing closure-based clock so the failure proves the quantization bug rather than missing rs-clock wiring:

```rust
#[test]
fn test_qubit_try_next_id_detects_raw_rollback_inside_second_slice() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let now = Arc::new(Mutex::new(epoch + Duration::from_millis(10_500)));
    let clock_now = Arc::clone(&now);
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .clock(move || {
            *clock_now.lock().expect("test clock lock should be available")
        })
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.try_next_id(),
        Ok(GenerationOutcome::Generated(_))
    ));
    *now.lock().expect("test clock lock should be available") =
        epoch + Duration::from_millis(10_400);

    assert_eq!(
        generator
            .try_next_id()
            .expect("small rollback should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(100))
    );
}
```

- [ ] **Step 2: Run only the regression and verify the behavioral failure**

Run:

```bash
cargo test --test mod test_qubit_try_next_id_detects_raw_rollback_inside_second_slice
```

Expected: assertion failure because both wall times quantize to timestamp `10`, so the current implementation incorrectly returns `Generated`.

- [ ] **Step 3: Add error-model tests before changing `IdError`**

Add pattern and source checks:

```rust
#[test]
fn test_id_error_clock_moved_backwards_preserves_raw_durations() {
    let error = IdError::ClockMovedBackwards {
        last_elapsed: Duration::from_millis(10_500),
        current_elapsed: Duration::from_millis(10_400),
        skew: Duration::from_millis(100),
        max_skew: Duration::from_secs(3),
    };
    assert!(error.to_string().contains("100ms"));
}

#[test]
fn test_id_error_preserves_sources() {
    let random = IdError::RandomSourceUnavailable {
        source: getrandom::Error::UNSUPPORTED,
    };
    assert!(std::error::Error::source(&random).is_some());

    let sleep = IdError::SleepFailed {
        source: qubit_clock::TimeError::InstantOverflow,
    };
    assert!(std::error::Error::source(&sleep).is_some());
}
```

Run `cargo test --test mod id_error_tests`; expected: compilation fails on the new fields and variants.

- [ ] **Step 4: Add dependencies and implement the contextual error enum**

Add:

```toml
qubit-clock = { version = "0.9", path = "../rs-clock" }
thiserror = "2.0"
```

Replace manual `Display`/`Error` implementations with `#[derive(Debug, Error)]`, retain all range variants, and use these exact reliability variants:

```rust
#[error(
    "clock moved backwards from {last_elapsed:?} to {current_elapsed:?}; \
     skew {skew:?} exceeds maximum {max_skew:?}"
)]
ClockMovedBackwards {
    /// Greatest elapsed time observed by the generator.
    last_elapsed: Duration,
    /// Elapsed time reported by the current wall-clock observation.
    current_elapsed: Duration,
    /// Difference between the last and current elapsed times.
    skew: Duration,
    /// Maximum tolerated backwards movement.
    max_skew: Duration,
},

#[error("time {time:?} is before the configured epoch {epoch:?}")]
TimeBeforeEpoch {
    /// Wall time that could not be represented relative to the epoch.
    time: SystemTime,
    /// Configured epoch or Sonyflake start time.
    epoch: SystemTime,
},

#[error(
    "start time {start_time:?} is ahead of generator clock {current_time:?}"
)]
StartTimeAhead {
    /// Configured Sonyflake start time.
    start_time: SystemTime,
    /// Wall time observed while validating the builder.
    current_time: SystemTime,
},

#[error("operating system random source is unavailable")]
RandomSourceUnavailable {
    /// Error returned by `getrandom`.
    #[source]
    source: getrandom::Error,
},

#[error("failed to wait before retrying ID generation")]
SleepFailed {
    /// Error returned by the injected blocking sleeper.
    #[source]
    source: qubit_clock::TimeError,
},
```

Mark the enum `#[non_exhaustive]`. Remove `Clone`, `Eq`, and `PartialEq`; update tests to use `matches!`, field assertions, messages, and `Error::source`.

- [ ] **Step 5: Implement raw `ClockObservation`**

Use `SystemTime` only at the boundary and preserve raw `Duration` before quantization:

```rust
pub(crate) struct ClockObservation {
    /// Unquantized duration since the configured reference time.
    pub(crate) elapsed: Duration,
    /// Encoded logical time slice.
    pub(crate) timestamp: u64,
    /// Positive duration from this observation to the next slice boundary.
    pub(crate) retry_after: Duration,
}

impl ClockObservation {
    /// Converts one wall time into raw and quantized generator time.
    pub(crate) fn from_time(
        time: SystemTime,
        epoch: SystemTime,
        time_unit: Duration,
        max_timestamp: u64,
    ) -> Result<Self, IdError> {
        debug_assert!(!time_unit.is_zero());
        let elapsed = time
            .duration_since(epoch)
            .map_err(|_| IdError::TimeBeforeEpoch { time, epoch })?;
        let unit_nanos = time_unit.as_nanos();
        let timestamp = elapsed.as_nanos() / unit_nanos;
        if timestamp > u128::from(max_timestamp) {
            return Err(IdError::TimestampOverflow {
                timestamp: u64::try_from(timestamp).unwrap_or(u64::MAX),
                max: max_timestamp,
            });
        }
        let elapsed_in_slice = elapsed.as_nanos() % unit_nanos;
        let retry_after = duration_from_nanos(
            unit_nanos - elapsed_in_slice,
        );
        Ok(Self {
            elapsed,
            timestamp: timestamp as u64,
            retry_after,
        })
    }
}

/// Converts a representable nanosecond count to `Duration`.
fn duration_from_nanos(nanos: u128) -> Duration {
    const NANOS_PER_SECOND: u128 = 1_000_000_000;
    let seconds = nanos / NANOS_PER_SECOND;
    debug_assert!(seconds <= u128::from(u64::MAX));
    Duration::new(
        seconds as u64,
        (nanos % NANOS_PER_SECOND) as u32,
    )
}
```

- [ ] **Step 6: Implement the shared generation state**

Implement the restart fence as:

```rust
pub(crate) enum RestartFence {
    /// Allocation may begin immediately.
    Disabled,
    /// The first timestamp has not yet been observed.
    Uninitialized,
    /// Allocation is waiting for a timestamp after the baseline.
    Waiting {
        /// Timestamp observed by the first generation attempt.
        baseline_timestamp: u64,
    },
}

impl RestartFence {
    /// Creates a fence for the configured restart policy.
    pub(crate) const fn new(policy: RestartPolicy) -> Self {
        match policy {
            RestartPolicy::Immediate => Self::Disabled,
            RestartPolicy::WaitNextSlice => Self::Uninitialized,
        }
    }

    /// Returns whether allocation must still wait at `timestamp`.
    pub(crate) fn should_wait(&mut self, timestamp: u64) -> bool {
        match *self {
            Self::Disabled => false,
            Self::Uninitialized => {
                *self = Self::Waiting {
                    baseline_timestamp: timestamp,
                };
                true
            }
            Self::Waiting { baseline_timestamp }
                if timestamp <= baseline_timestamp => true,
            Self::Waiting { .. } => {
                *self = Self::Disabled;
                false
            }
        }
    }
}
```

Define the state and constructor before its transition method:

```rust
pub(crate) struct GenerationState {
    /// Greatest raw elapsed time observed without rollback.
    last_observed_elapsed: Option<Duration>,
    /// Last allocated logical slice and sequence.
    time_slice: Option<TimeSlice>,
    /// First-allocation restart fence.
    restart_fence: RestartFence,
}

impl GenerationState {
    /// Creates empty allocation state for `restart_policy`.
    pub(crate) const fn new(restart_policy: RestartPolicy) -> Self {
        Self {
            last_observed_elapsed: None,
            time_slice: None,
            restart_fence: RestartFence::new(restart_policy),
        }
    }
}
```

`GenerationState::reserve` must use this order:

```rust
pub(crate) fn reserve(
    &mut self,
    observation: ClockObservation,
    max_sequence: u64,
    max_clock_skew: Duration,
) -> Result<GenerationOutcome<TimeSlice>, IdError> {
    if let Some(last_elapsed) = self.last_observed_elapsed {
        if observation.elapsed < last_elapsed {
            let skew = last_elapsed - observation.elapsed;
            if skew > max_clock_skew {
                return Err(IdError::ClockMovedBackwards {
                    last_elapsed,
                    current_elapsed: observation.elapsed,
                    skew,
                    max_skew: max_clock_skew,
                });
            }
            return Ok(GenerationOutcome::RetryAfter(skew));
        }
    }
    self.last_observed_elapsed = Some(observation.elapsed);

    if self.restart_fence.should_wait(observation.timestamp) {
        return Ok(GenerationOutcome::RetryAfter(
            observation.retry_after,
        ));
    }

    let Some(time_slice) = self.time_slice.as_mut() else {
        let time_slice = TimeSlice::new(observation.timestamp);
        self.time_slice = Some(time_slice);
        return Ok(GenerationOutcome::Generated(time_slice));
    };
    if observation.timestamp > time_slice.timestamp {
        *time_slice = TimeSlice::new(observation.timestamp);
        return Ok(GenerationOutcome::Generated(*time_slice));
    }
    debug_assert_eq!(observation.timestamp, time_slice.timestamp);
    if time_slice.sequence == max_sequence {
        return Ok(GenerationOutcome::RetryAfter(
            observation.retry_after,
        ));
    }
    time_slice.sequence += 1;
    Ok(GenerationOutcome::Generated(*time_slice))
}
```

`GenerationState::new(RestartPolicy::Immediate)` initializes a disabled fence;
`WaitNextSlice` initializes an unobserved fence. Keep the wall-clock read and
`reserve` call inside the same generator mutex critical section.

- [ ] **Step 7: Migrate all three generators to raw observations**

For each generator, replace `Mutex<Option<TimeSlice>>` with
`Mutex<GenerationState>`. Build a `ClockObservation` with these units:

```rust
// Qubit
Duration::from_millis(self.layout.precision().divisor_millis())

// Classic Snowflake
Duration::from_millis(1)

// Sonyflake
self.time_unit
```

Qubit passes `Duration::from_millis(self.max_skew_millis)` temporarily;
Classic and Sonyflake pass `Duration::ZERO`. Compose only a
`GenerationOutcome::Generated(time_slice)` and preserve `RetryAfter` exactly.
Remove `wait_duration_millis`, `WAIT_DURATION_IN_MILLISECOND`,
`WAIT_DURATION_IN_SECOND`, the old reservation function, and the obsolete
external time-slice test. Update Mica to preserve the `getrandom::Error` source:

```rust
getrandom::fill(&mut bytes)
    .map_err(|source| IdError::RandomSourceUnavailable { source })?;
```

Update the existing Sonyflake builder validation at the same checkpoint so the
new contextual error compiles before rs-clock injection:

```rust
let current_time = (clock)();
if start_time > current_time {
    return Err(IdError::StartTimeAhead {
        start_time,
        current_time,
    });
}
```

- [ ] **Step 8: Run regression, error, generator, and full tests**

Run:

```bash
cargo test --test mod test_qubit_try_next_id_detects_raw_rollback_inside_second_slice
cargo test --test mod id_error_tests
cargo test --test mod qubit_snowflake_generator_tests
cargo test --test mod snowflake_generator_tests
cargo test --test mod sonyflake_generator_tests
cargo test
```

Expected: all pass; the regression returns `RetryAfter(100ms)`, fatal rollback fields contain raw durations, and no layout test changes its encoded value.

- [ ] **Step 9: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add Cargo.toml Cargo.lock src tests
git commit -m "fix(id): 在量化前检测时钟回拨"
```

---

### Task 3: Add Qubit restart policy and rs-clock injection

**Files:**
- Create: `src/snowflake/internal/block_until_generated.rs`
- Create: `src/snowflake/internal/clock_defaults.rs`
- Modify: `src/snowflake/internal/mod.rs`
- Modify: `src/snowflake/constants.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/qubit_snowflake_generator_builder.rs`
- Move: `tests/snowflake/qubit_snowflake_generator/qubit_snowflake_generator_tests.rs` to `tests/snowflake/qubit_snowflake_generator_tests.rs`
- Modify: `tests/snowflake/qubit_snowflake_generator_builder_tests.rs`
- Create: `tests/support/manual_time.rs`
- Create: `tests/support/failing_blocking_sleeper.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/snowflake/mod.rs`
- Delete: `tests/snowflake/qubit_snowflake_generator/mod.rs`

**Interfaces:**
- Consumes: `GenerationState::new(policy)` and exact retry durations from Task 2.
- Produces: Qubit builder methods `restart_policy`, `wall_clock`, `blocking_sleeper`, and `max_clock_skew(Duration)`.

- [ ] **Step 1: Add failing Qubit policy and clock-injection tests**

Cover default `Immediate`, reproducible same-slice duplicates, the complete
`WaitNextSlice` transition, non-blocking sequence exhaustion, deterministic
blocking wake-up, and sleeper source propagation. The central restart test is:

```rust
#[test]
fn test_qubit_wait_next_slice_delays_first_allocation() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(
        generator.try_next_id().expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(750))
    );
    time.advance(Duration::from_millis(749));
    assert_eq!(
        generator.try_next_id().expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(1))
    );
    time.advance(Duration::from_millis(1));
    let id = match generator
        .try_next_id()
        .expect("next slice should allocate")
    {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}
```

Run the focused tests; expected: compilation fails because the new builder
methods and `ManualTime` fixture do not exist.

- [ ] **Step 2: Implement deterministic test time support**

Create one fixture using a shared manual monotonic clock:

```rust
pub(crate) struct ManualTime {
    /// Monotonic timeline advanced by the test driver.
    monotonic_clock: Arc<ManualMonotonicClock>,
    /// Wall-time projection used by generators.
    wall_clock: Arc<ManualWallClock>,
    /// Blocking sleeper registered on the same monotonic timeline.
    blocking_sleeper: Arc<ManualBlockingSleeper>,
}

impl ManualTime {
    /// Creates a fixture whose wall clock initially reads `now`.
    pub(crate) fn new(now: SystemTime) -> Self {
        let monotonic_clock = Arc::new(ManualMonotonicClock::new());
        let wall_clock = Arc::new(ManualWallClock::from_clock(
            now,
            Arc::clone(&monotonic_clock),
        ));
        let blocking_sleeper = Arc::new(
            ManualBlockingSleeper::from_clock(Arc::clone(&monotonic_clock)),
        );
        Self {
            monotonic_clock,
            wall_clock,
            blocking_sleeper,
        }
    }

    /// Returns the wall clock as its public trait object.
    pub(crate) fn wall_clock(&self) -> Arc<dyn WallClock> {
        self.wall_clock.clone()
    }

    /// Returns the sleeper as its public trait object.
    pub(crate) fn blocking_sleeper(&self) -> Arc<dyn BlockingSleeper> {
        self.blocking_sleeper.clone()
    }

    /// Reanchors wall time without moving monotonic time.
    pub(crate) fn reanchor(&self, now: SystemTime) {
        self.wall_clock.reanchor(now);
    }

    /// Advances both monotonic time and the wall-time projection.
    pub(crate) fn advance(&self, duration: Duration) {
        self.monotonic_clock
            .advance(duration)
            .expect("manual time should advance");
    }

    /// Waits for one sleeper deadline and advances directly to it.
    pub(crate) fn advance_to_next_deadline(&self) {
        assert!(
            self.monotonic_clock
                .wait_for_waiters(1, Duration::from_secs(1)),
            "blocking generator should register a deadline"
        );
        self.monotonic_clock
            .advance_to_next_deadline()
            .expect("a future deadline should be registered");
    }
}
```

Implement the failing sleeper used by the error-path test:

```rust
pub(crate) struct FailingBlockingSleeper {
    /// Monotonic clock returned by the sleeper contract.
    clock: StdMonotonicClock,
}

impl FailingBlockingSleeper {
    /// Creates a sleeper that always reports instant overflow.
    pub(crate) fn new() -> Self {
        Self {
            clock: StdMonotonicClock::new(),
        }
    }
}

impl BlockingSleeper for FailingBlockingSleeper {
    /// Returns the fixture's monotonic clock.
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Returns a stable error without blocking.
    fn sleep_until(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<(), TimeError> {
        Err(TimeError::InstantOverflow)
    }
}
```

- [ ] **Step 3: Implement standard clock defaults and blocking adaptation**

```rust
pub(crate) fn default_wall_clock() -> Arc<dyn WallClock> {
    Arc::new(StdWallClock::new())
}

pub(crate) fn default_blocking_sleeper() -> Arc<dyn BlockingSleeper> {
    let clock = Arc::new(StdMonotonicClock::new());
    Arc::new(StdBlockingSleeper::from_clock(clock))
}
```

```rust
pub(crate) fn block_until_generated<T, F>(
    sleeper: &dyn BlockingSleeper,
    mut attempt: F,
) -> Result<T, IdError>
where
    F: FnMut() -> Result<GenerationOutcome<T>, IdError>,
{
    loop {
        match attempt()? {
            GenerationOutcome::Generated(id) => return Ok(id),
            GenerationOutcome::RetryAfter(duration) => {
                sleeper
                    .sleep_for(duration)
                    .map_err(|source| IdError::SleepFailed { source })?;
            }
        }
    }
}
```

- [ ] **Step 4: Refactor Qubit configuration to rs-clock types**

Replace `DEFAULT_MAX_SKEW_MILLIS` with:

```rust
pub const DEFAULT_MAX_CLOCK_SKEW: Duration = Duration::from_secs(3);
```

The builder stores:

```rust
max_clock_skew: Duration,
restart_policy: RestartPolicy,
wall_clock: Arc<dyn WallClock>,
blocking_sleeper: Arc<dyn BlockingSleeper>,
```

Add setters with direct assignment and `#[must_use]`:

```rust
pub fn max_clock_skew(mut self, max_clock_skew: Duration) -> Self {
    self.max_clock_skew = max_clock_skew;
    self
}

pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
    self.restart_policy = restart_policy;
    self
}

pub fn wall_clock(mut self, wall_clock: Arc<dyn WallClock>) -> Self {
    self.wall_clock = wall_clock;
    self
}

pub fn blocking_sleeper(
    mut self,
    blocking_sleeper: Arc<dyn BlockingSleeper>,
) -> Self {
    self.blocking_sleeper = blocking_sleeper;
    self
}
```

Remove the closure `clock` setter. Pass the policy, clocks and `Duration` into
`from_config`; initialize `GenerationState::new(restart_policy)`.

- [ ] **Step 5: Make Qubit blocking generation use the injected sleeper**

Read `wall_clock.now()` while holding the state mutex in `try_next_id()`. Keep
`try_next_id()` free of sleeper calls. Implement blocking generation as:

```rust
fn next_id(&self) -> Result<Self::Id, Self::Error> {
    block_until_generated(self.blocking_sleeper.as_ref(), || {
        self.try_next_id()
    })
}
```

Expose `max_clock_skew(&self) -> Duration`; remove `max_skew_millis`.

- [ ] **Step 6: Verify Qubit blocking and error-source behavior**

For blocking `WaitNextSlice`, run `next_id()` on a worker thread, call
`ManualTime::advance_to_next_deadline()` on the driver, and assert sequence
zero in the next slice. For `SleepFailed`, inject `FailingBlockingSleeper`, use
`WaitNextSlice` with a fixed wall time, call `next_id()`, and match
`IdError::SleepFailed { source: TimeError::InstantOverflow }`.

Run:

```bash
cargo test --test mod qubit_snowflake_generator_tests
cargo test --test mod qubit_snowflake_generator_builder_tests
cargo test
```

Expected: all pass without real sleeps; default `Immediate` remains immediate.

- [ ] **Step 7: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add Cargo.toml Cargo.lock src tests
git commit -m "feat(id): 为 Qubit 增加重启策略和可注入时钟"
```

---

### Task 4: Migrate Classic Snowflake to the shared builder and clocks

**Files:**
- Create: `src/snowflake/snowflake_generator_builder.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/lib.rs`
- Move: `tests/snowflake/snowflake_generator/snowflake_generator_tests.rs` to `tests/snowflake/snowflake_generator_tests.rs`
- Create: `tests/snowflake/snowflake_generator_builder_tests.rs`
- Modify: `tests/snowflake/mod.rs`
- Delete: `tests/snowflake/snowflake_generator/mod.rs`

**Interfaces:**
- Consumes: common clock defaults, `block_until_generated`, `ManualTime`, and `RestartPolicy`.
- Produces: `SnowflakeGenerator::builder(node_id)` plus restart/clock/sleeper configuration.

- [ ] **Step 1: Add failing Classic builder and restart tests**

Test these behaviors:

```rust
#[test]
fn test_snowflake_wait_next_slice_delays_first_allocation() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(
        epoch + Duration::from_micros(10_250),
    );
    let generator = SnowflakeGenerator::builder(11)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(
        generator.try_next_id().expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_micros(750))
    );
    time.advance(Duration::from_micros(750));
    let id = match generator.try_next_id().expect("next slice should allocate") {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    assert_eq!(generator.extract_timestamp(id), 11);
    assert_eq!(generator.extract_sequence(id), 0);
}
```

Also test two default `Immediate` instances returning the same ID at the same
fixed time, exact sequence-exhaustion retry, deterministic blocking wake-up,
and a 100-microsecond rollback inside one encoded millisecond returning
`ClockMovedBackwards`.

- [ ] **Step 2: Run focused tests and verify missing builder API failures**

Run:

```bash
cargo test --test mod snowflake_generator_builder_tests
cargo test --test mod snowflake_generator_tests
```

Expected: compilation fails because Classic does not yet expose the builder or
rs-clock setters.

- [ ] **Step 3: Implement `SnowflakeGeneratorBuilder`**

The builder fields are:

```rust
pub struct SnowflakeGeneratorBuilder {
    /// Node identifier encoded in generated IDs.
    node_id: u64,
    /// Timestamp origin.
    epoch: SystemTime,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled by the generator.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
}
```

`new(node_id)` uses the existing default Qubit epoch, `Immediate`, and the
shared standard clock factories. Provide `epoch`, `restart_policy`,
`wall_clock`, `blocking_sleeper`, and `build`. Validate `node_id <= 1023` in
`build` or `from_config` and return `IdError::NodeOutOfRange` otherwise.

- [ ] **Step 4: Make the generator builder-only for custom configuration**

Keep:

```rust
pub fn new(node_id: u64) -> Result<Self, IdError> {
    Self::builder(node_id).build()
}

#[must_use]
pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
    SnowflakeGeneratorBuilder::new(node_id)
}
```

Remove public `with_epoch` and `with_clock`. Store the two rs-clock trait
objects, initialize `GenerationState` from the policy, read the wall clock
inside the mutex, and delegate `next_id()` to `block_until_generated`.

- [ ] **Step 5: Run Classic tests and the full suite**

Run:

```bash
cargo test --test mod snowflake_generator_builder_tests
cargo test --test mod snowflake_generator_tests
cargo test
```

Expected: all pass; no Classic test uses a closure clock or real sleep.

- [ ] **Step 6: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add src tests
git commit -m "feat(id): 统一 Classic Snowflake 构建与等待语义"
```

---

### Task 5: Migrate Sonyflake to the shared policy and clocks

**Files:**
- Modify: `src/snowflake/sonyflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator_builder.rs`
- Move: `tests/snowflake/sonyflake_generator/sonyflake_generator_tests.rs` to `tests/snowflake/sonyflake_generator_tests.rs`
- Modify: `tests/snowflake/sonyflake_generator_builder_tests.rs`
- Modify: `tests/snowflake/mod.rs`
- Delete: `tests/snowflake/sonyflake_generator/mod.rs`

**Interfaces:**
- Consumes: the shared generation state, clock defaults, blocking adapter, and `ManualTime`.
- Produces: Sonyflake restart/clock/sleeper builder configuration with zero rollback tolerance.

- [ ] **Step 1: Add failing Sonyflake policy, rollback, and blocking tests**

Use a two-bit sequence field where exhaustion needs only four allocations. The
restart test must observe a partial 10 ms unit and expect the exact remainder:

```rust
#[test]
fn test_sonyflake_wait_next_slice_delays_first_allocation() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(25));
    let generator = SonyflakeGenerator::builder(13)
        .bits_sequence(2)
        .time_unit(Duration::from_millis(10))
        .start_time(start_time)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(
        generator.try_next_id().expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(5))
    );
    time.advance(Duration::from_millis(5));
    let id = match generator.try_next_id().expect("next unit should allocate") {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    assert_eq!(generator.extract_elapsed_time(id), 3);
    assert_eq!(generator.extract_sequence(id), 0);
}
```

Add a rollback test that generates at 25 ms, reanchors to 24 ms, and expects a
one-millisecond `ClockMovedBackwards` even though both values encode unit 2.

- [ ] **Step 2: Run focused tests and verify missing setters**

Run:

```bash
cargo test --test mod sonyflake_generator_builder_tests
cargo test --test mod sonyflake_generator_tests
```

Expected: compilation fails because the Sonyflake builder still accepts a
closure and has no restart or sleeper configuration.

- [ ] **Step 3: Replace closure clocks in the Sonyflake builder and generator**

Store `restart_policy`, `Arc<dyn WallClock>`, and
`Arc<dyn BlockingSleeper>` in the builder. Default to `Immediate` and the
shared standard factories. Replace `clock` with `wall_clock` and
`blocking_sleeper` setters.

During build validation, sample once and retain both times in the error:

```rust
let current_time = wall_clock.now();
if start_time > current_time {
    return Err(IdError::StartTimeAhead {
        start_time,
        current_time,
    });
}
```

Initialize `GenerationState::new(restart_policy)`, read the wall clock in the
mutex, pass `Duration::ZERO` as maximum skew, and use
`block_until_generated` for `next_id()`.

- [ ] **Step 4: Run Sonyflake and full tests**

Run:

```bash
cargo test --test mod sonyflake_generator_builder_tests
cargo test --test mod sonyflake_generator_tests
cargo test
```

Expected: all pass; Sonyflake detects sub-unit raw rollback and all waits are
driven by the injected sleeper.

- [ ] **Step 5: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add src tests
git commit -m "feat(id): 统一 Sonyflake 重启与时钟策略"
```

---

### Task 6: Finish Rust source, test, and benchmark organization

**Files:**
- Modify: every Rust file changed by Tasks 1–5 for rustdoc, import, method-order, and attribute checks
- Modify: `tests/mod.rs`
- Modify: `tests/snowflake/mod.rs`
- Modify: `tests/uuid/mod.rs`
- Delete: obsolete nested test directories after their contents have moved
- Modify: `benches/qubit_snowflake_throughput.rs`
- Create: `benches/qubit_snowflake_throughput/throughput_sample.rs`
- Create: `benches/qubit_snowflake_throughput/throughput_summary.rs`
- Create: `benches/qubit_snowflake_throughput/startup_latency_summary.rs`

**Interfaces:**
- Preserves: every public signature and behavior completed in Tasks 1–5.
- Produces: project-style-compliant source, exact source/test path mapping, and an unchanged benchmark algorithm.

- [ ] **Step 1: Run format and project style checks before cleanup**

Run:

```bash
cargo fmt -- --check
./style-check.sh
```

Expected: any remaining moved-module, explicit-import, aggregation-file, or
format violations are reported with file paths; record each reported path in
the task notes before editing.

- [ ] **Step 2: Complete test path mapping and remove obsolete modules**

Ensure `tests/mod.rs` directly declares:

```rust
mod generation_outcome_tests;
mod id_error_tests;
mod id_generator_tests;
mod snowflake;
mod support;
mod uuid;
```

Ensure `tests/snowflake/mod.rs` declares only flat `*_tests` modules matching
the source filenames, including `restart_policy_tests` and
`snowflake_generator_builder_tests`. Ensure `tests/uuid/mod.rs` directly
declares `mica_uuid_like_generator_tests`. Delete empty nested directories and
all redirection-only `mod.rs` files.

- [ ] **Step 3: Normalize Rust docs, imports, method order, and attributes**

For each changed Rust file:

- use explicit imports and keep aggregation-only `mod.rs` files free of private imports;
- document all fields and all private/test helper functions;
- use `# Arguments`, `# Returns`, `# Errors`, `# Panics`, and blocking notes where applicable;
- order inherent methods as constructor/builder, public accessors and operations, restricted methods, then private helpers;
- place `#[must_use]` and `#[inline]` according to the project style rules;
- keep one top-level type per file and do not introduce allowlist exceptions.

No behavior or public naming change is permitted in this step.

- [ ] **Step 4: Split benchmark types without changing measurement code**

Move the three types into their own module files. For example:

```rust
#[derive(Clone, Copy)]
pub(super) struct ThroughputSample {
    /// Number of IDs generated in the observation.
    pub(super) generated: u64,
    /// Theoretical sequence capacity for the measured slices.
    pub(super) capacity: u64,
    /// Wall duration of the observation.
    pub(super) elapsed: Duration,
}

impl ThroughputSample {
    /// Returns the percentage of theoretical sequence capacity consumed.
    pub(super) fn utilization(self) -> f64 {
        self.generated as f64 * 100.0 / self.capacity as f64
    }

    /// Returns IDs generated per elapsed second.
    pub(super) fn throughput(self) -> f64 {
        self.generated as f64 / self.elapsed.as_secs_f64()
    }
}
```

`ThroughputSummary` contains three `ThroughputSample` fields and
`StartupLatencySummary` contains the three nanosecond fields, all
`pub(super)`. Declare these modules in the benchmark root and import the exact
types. Do not change constants, worker loops, capacity assertions, timing, or
printed output.

- [ ] **Step 5: Format and verify source organization**

Run:

```bash
cargo fmt
cargo check --benches
./style-check.sh
cargo test
```

Expected: all commands pass and no obsolete nested test module remains.

- [ ] **Step 6: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add src tests benches
git commit -m "style(id): 整理 Rust 类型与测试目录"
```

---

### Task 7: Synchronize English, Chinese, and rustdoc documentation

**Files:**
- Modify: `README.md`
- Modify: `README.zh_CN.md`
- Modify: `src/lib.rs`
- Modify: `src/id_generator.rs`
- Modify: `src/generation_outcome.rs`
- Modify: `src/snowflake/restart_policy.rs`
- Modify: `src/snowflake/qubit_snowflake_generator.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`
- Modify: `src/snowflake/qubit_snowflake_layout.rs`

**Interfaces:**
- Documents: exact API and limitations already implemented; no behavior change.

- [ ] **Step 1: Update README examples to show both generation modes**

The English example must include this control flow, with an equivalent Chinese
example:

```rust
match generator.try_next_id()? {
    GenerationOutcome::Generated(id) => println!("{id}"),
    GenerationOutcome::RetryAfter(duration) => {
        println!("retry after {duration:?}");
    }
}
```

Add a `WaitNextSlice` builder example and state that the default is
`Immediate`.

- [ ] **Step 2: Document exact duplicate and blocking conditions**

In both READMEs and all three generator rustdocs, state all three duplicate
preconditions: same effective identity/layout/reference time, same logical
time slice, and overlapping sequence ranges. State that `WaitNextSlice`
protects only sequential replacement and does not coordinate concurrent
same-identity instances.

State that `try_next_id()` never sleeps. State that `next_id()` may wait
indefinitely when the wall clock stalls or an injected sleeper does not cause
the wall clock to progress.

- [ ] **Step 3: Document signed, version, and decode limitations verbatim in both languages**

English requirements:

```markdown
Qubit Spread IDs always set bit 63 and therefore always exceed `i64::MAX`.
Store them as unsigned 64-bit values, decimal strings, or binary data; use
strings when crossing JavaScript-style safe-integer boundaries.

The 64-bit layout reserves neither a sign bit nor a version field. This is an
intentional capacity and throughput trade-off. A future incompatible layout
must use a new explicit type or API rather than silently changing this one.

Decoding an arbitrary `u64` only extracts fields according to the layout. It
does not prove that the value was produced by this generator and is not an
authenticity or format-validation operation.
```

Add a semantically equivalent Chinese passage next to the corresponding
Chinese layout documentation.

- [ ] **Step 4: Build docs and validate README dependency versions**

Run:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --doc
python3 .rs-ci/readme-version-check.py
```

Expected: all pass; English and Chinese examples use `qubit-id = "0.3"` as
required by the current package metadata.

- [ ] **Step 5: Create an approval-gated checkpoint**

Only after explicit Git authorization:

```bash
git add README.md README.zh_CN.md src
git commit -m "docs(id): 说明重启策略与布局限制"
```

---

### Task 8: Run full verification and refresh the performance evidence

**Files:**
- Modify: `docs/benchmarks/qubit-snowflake-throughput.md` only with actual benchmark output
- Inspect: all files changed in Tasks 1–7

**Interfaces:**
- Produces: verified implementation evidence and a separate mutex-performance decision point for the user.

- [ ] **Step 1: Inspect the complete change set before verification**

Run:

```bash
git status --short
git --no-pager diff --check
git --no-pager diff -- Cargo.toml src tests README.md README.zh_CN.md benches docs/benchmarks
```

Expected: only task-scoped files and the approved spec/plan are changed;
`git diff --check` prints no whitespace errors.

- [ ] **Step 2: Run targeted public-behavior tests**

Run:

```bash
cargo test --test mod generation_outcome_tests
cargo test --test mod id_generator_tests
cargo test --test mod id_error_tests
cargo test --test mod restart_policy_tests
cargo test --test mod qubit_snowflake_generator_tests
cargo test --test mod snowflake_generator_tests
cargo test --test mod sonyflake_generator_tests
```

Expected: every command passes with deterministic time and no network access.

- [ ] **Step 3: Run the project-level verification suite**

Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
./style-check.sh
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo check --benches
./ci-check.sh
```

Expected: all commands exit zero. `./ci-check.sh` additionally verifies the
Rust 1.94 build, release build, package, coverage thresholds, feature matrix,
README versions, security audit, and pinned lint toolchain.

- [ ] **Step 4: Run the existing throughput benchmark without changing its method**

Run:

```bash
RUSTFLAGS="-C target-cpu=native" \
    cargo bench --bench qubit_snowflake_throughput
```

Copy the emitted configuration, eight sustained-throughput rows, and two
startup-latency rows into `docs/benchmarks/qubit-snowflake-throughput.md`.
Update the working-tree description to identify this reliability refactor,
retain the exact machine/toolchain information, and explain any observed
startup-cost change from allocating standard clock trait objects. Do not claim
a regression or improvement from a difference smaller than the three-sample
min-to-max ranges.

- [ ] **Step 5: Re-run documentation and diff checks after recording results**

Run:

```bash
git --no-pager diff --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Expected: both commands pass.

- [ ] **Step 6: Present the deferred mutex question separately**

Report the final single-thread and multi-thread benchmark medians, remind the
user that the shared mutex was deliberately retained, and ask whether to open
a separate design cycle comparing the current mutex, batch sequence leases,
and an atomic state machine. Do not implement any of those performance options
in this plan.

- [ ] **Step 7: Create an approval-gated final checkpoint**

Only after explicit Git authorization:

```bash
git add docs/benchmarks/qubit-snowflake-throughput.md
git commit -m "perf(id): 更新生成可靠性重构基准"
```
