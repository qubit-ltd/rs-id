# rs-id API、Feature 与寿命治理 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 默认只发布 Qubit Snowflake，按 feature 提供其余算法，并完成 Layout/Parts、构造期寿命保护、must-use/inline 审计和 UUID 对比 benchmark。

**Architecture:** `IdGenerator`、`GenerationOutcome` 和 `IdError` 保持核心层；各算法由独立 Cargo feature 控制。Classic 与 Sonyflake 的位布局迁入独立 Layout/Parts 类型，生成器只持有布局、时间原点、缓存到期时间和分配状态；所有 Snowflake builder 通过共享 checked 时间计算得到排他的 `expires_at`，并在当前时钟已经到期时 panic。

**Tech Stack:** Rust 1.94、edition 2024、Cargo features、`thiserror` 2、`qubit-clock` 0.9、`parking_lot` 0.12、`getrandom` 0.4、`uuid` 1.x dev-dependency。

## Global Constraints

- 默认 feature 精确为 `qubit-snowflake`；可选 feature 为 `classic-snowflake`、`sonyflake`、`uuid`。
- `--no-default-features` 只编译核心 trait/outcome/error；所有 Snowflake feature 启用 `parking_lot` 与 `qubit-clock`，`uuid` 单独启用 `getrandom`。
- 允许破坏性 API 变更，不保留 Classic/Sonyflake generator 上的 compose/extract 兼容层。
- 到期时间是排他边界；`now >= expires_at` 时 builder panic，边界计算本身无法表示时返回 `IdError::ExpirationTimeOverflow`。
- 每个 Rust 类型独占 snake-case 文件；测试只放在镜像 `tests/` 路径；新 Rust 文件复制仓库完整版权头。
- 所有新增/修改 API 保持英文 rustdoc，并完整说明 Arguments、Returns、Errors、Panics 与必要示例。
- 严格执行 RED → GREEN → REFACTOR；生产行为代码之前必须观察到目标测试按预期失败。
- 未经用户明确要求不执行 `git add` 或 `git commit`；每个任务以测试结果和 `git diff --check` 作为检查点。

---

### Task 1: 建立 Cargo feature 与条件模块边界

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/id_error.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `tests/mod.rs`
- Modify: `tests/id_error_tests.rs`
- Modify: `tests/snowflake/mod.rs`
- Modify: `tests/markdown_tests/readme_examples_tests.rs`

**Interfaces:**
- Produces: features `qubit-snowflake`、`classic-snowflake`、`sonyflake`、`uuid`；默认仅 `qubit-snowflake`。
- Produces: feature-safe `IdError`；随机源和 sleeper source variant 仅在依赖存在时编译。

- [x] **Step 1: 运行不存在 feature 的 RED 命令**

Run:

```bash
cargo check --no-default-features --features qubit-snowflake
```

Expected: FAIL，错误说明 package 尚无 `qubit-snowflake` feature。

- [x] **Step 2: 写入最小 feature 与 dependency 配置**

```toml
[features]
default = ["qubit-snowflake"]
qubit-snowflake = ["dep:parking_lot", "dep:qubit-clock"]
classic-snowflake = ["dep:parking_lot", "dep:qubit-clock"]
sonyflake = ["dep:parking_lot", "dep:qubit-clock"]
uuid = ["dep:getrandom"]

[dependencies]
getrandom = { version = "0.4", optional = true }
parking_lot = { version = "0.12", optional = true }
qubit-clock = { version = "0.9", path = "../rs-clock", optional = true }
thiserror = "2.0"
```

`src/lib.rs` 的算法模块使用：

```rust
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub mod snowflake;

#[cfg(feature = "uuid")]
pub mod uuid;
```

`src/snowflake/mod.rs` 中算法专属 module/re-export 只在对应 feature 下声明；
`restart_policy` 和 `internal` 在任意 Snowflake feature 下声明。测试 module 使用
相同 cfg。`RandomSourceUnavailable` 及其测试仅在 `uuid` 下存在，
`SleepFailed` 及其测试仅在任意 Snowflake feature 下存在。

- [x] **Step 3: 运行 GREEN feature 矩阵**

```bash
cargo check --no-default-features
cargo check --no-default-features --features qubit-snowflake
cargo check --no-default-features --features classic-snowflake
cargo check --no-default-features --features sonyflake
cargo check --no-default-features --features uuid
cargo check --all-features
```

Expected: all PASS。

- [x] **Step 4: 检查任务差异**

Run: `git --no-pager diff --check`

Expected: exit 0；不提交。

---

### Task 2: 提取 Classic Snowflake Layout 与 Parts

**Files:**
- Create: `src/snowflake/snowflake_layout.rs`
- Create: `src/snowflake/snowflake_parts.rs`
- Create: `tests/snowflake/snowflake_layout_tests.rs`
- Create: `tests/snowflake/snowflake_parts_tests.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/snowflake/snowflake_generator.rs`
- Modify: `src/snowflake/snowflake_generator_builder.rs`
- Modify: `tests/snowflake/mod.rs`
- Modify: `tests/snowflake/snowflake_generator_tests.rs`
- Modify: `tests/snowflake/snowflake_generator_builder_tests.rs`

**Interfaces:**
- Produces: `SnowflakeLayout::new(node_id)`、getter、`compose(timestamp, sequence)`、`decode(id)`。
- Produces: `SnowflakeParts` 及 `timestamp()`、`node_id()`、`sequence()`。
- Produces: `SnowflakeGenerator::layout()`；删除 generator 上 compose/extract/node/max API。

- [x] **Step 1: 写 Layout/Parts RED 测试**

```rust
#[test]
fn test_snowflake_layout_compose_decode_round_trip() {
    let layout = SnowflakeLayout::new(17)
        .expect("node id must fit the classic layout");
    let id = layout
        .compose(123_456, 789)
        .expect("parts must fit the classic layout");
    let parts = SnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 123_456);
    assert_eq!(parts.node_id(), 17);
    assert_eq!(parts.sequence(), 789);
}
```

另测 node、timestamp、sequence 最大值及越界错误，并测试 generator 的
`layout()` 与配置一致。

- [x] **Step 2: 运行 RED 测试**

```bash
cargo test --no-default-features --features classic-snowflake \
    snowflake_layout --verbose
```

Expected: FAIL，原因是 `SnowflakeLayout`/`SnowflakeParts` 尚不存在。

- [x] **Step 3: 实现 Layout/Parts 最小 API**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SnowflakeLayout {
    node_id: u64,
}

impl SnowflakeLayout {
    pub fn new(node_id: u64) -> Result<Self, IdError>;
    pub const fn node_id(&self) -> u64;
    pub const fn max_timestamp(&self) -> u64;
    pub const fn max_sequence(&self) -> u64;
    pub fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdError>;
    pub const fn decode(id: u64) -> SnowflakeParts;
}
```

`SnowflakeParts` 是独立 must-use 值类型，保存三个私有字段并提供 const getter。
generator 字段替换为 `layout: SnowflakeLayout`，生成路径调用
`self.layout.compose(timestamp, sequence)`；builder 先构造 layout。删除 generator
上的旧布局方法并同步既有测试与文档。

- [x] **Step 4: 运行 Classic GREEN 测试**

```bash
cargo test --no-default-features --features classic-snowflake --verbose
```

Expected: PASS，包含新 Layout/Parts 和既有回拨、并发、重启测试。

- [x] **Step 5: 检查任务差异**

Run: `git --no-pager diff --check`

Expected: exit 0；不提交。

---

### Task 3: 提取 Sonyflake Layout 与 Parts

**Files:**
- Create: `src/snowflake/sonyflake_layout.rs`
- Create: `src/snowflake/sonyflake_parts.rs`
- Create: `tests/snowflake/sonyflake_layout_tests.rs`
- Create: `tests/snowflake/sonyflake_parts_tests.rs`
- Modify: `src/snowflake/mod.rs`
- Modify: `src/snowflake/sonyflake_generator.rs`
- Modify: `src/snowflake/sonyflake_generator_builder.rs`
- Modify: `tests/snowflake/mod.rs`
- Modify: `tests/snowflake/sonyflake_generator_tests.rs`
- Modify: `tests/snowflake/sonyflake_generator_builder_tests.rs`

**Interfaces:**
- Produces: `SonyflakeLayout::new(machine_id, bits_sequence, bits_machine, time_unit)`、getter、max、compose/decode。
- Produces: `SonyflakeParts` 及 `elapsed_time()`、`sequence()`、`machine_id()`。
- Produces: `SonyflakeGenerator::layout()`；删除 generator 上旧布局 API。

- [x] **Step 1: 写 Sonyflake Layout/Parts RED 测试**

```rust
#[test]
fn test_sonyflake_layout_compose_decode_round_trip() {
    let layout = SonyflakeLayout::new(
        23,
        8,
        16,
        Duration::from_millis(10),
    )
    .expect("Sonyflake layout must be valid");
    let id = layout
        .compose(456_789, 31)
        .expect("parts must fit the Sonyflake layout");
    let parts = layout.decode(id);

    assert_eq!(parts.elapsed_time(), 456_789);
    assert_eq!(parts.sequence(), 31);
    assert_eq!(parts.machine_id(), 23);
}
```

另测 bit 总和、time bits 下限、最小 time unit、machine/sequence/time 越界与
generator `layout()`。

- [x] **Step 2: 运行 RED 测试**

```bash
cargo test --no-default-features --features sonyflake \
    sonyflake_layout --verbose
```

Expected: FAIL，原因是新类型尚不存在。

- [x] **Step 3: 实现 Layout/Parts 并迁移校验**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SonyflakeLayout {
    bits_time: u8,
    bits_sequence: u8,
    bits_machine: u8,
    time_unit: Duration,
    machine_id: u64,
}
```

将 `normalize_bits`、位宽推导、最小时间单位和 machine ID 校验迁入 Layout。
`compose(elapsed_time, sequence)` 固定使用 layout machine ID；`decode(&self, id)`
按配置位宽返回 Parts。generator 持有 layout，builder 先构造 layout，再处理
start time 与 wall clock。删除 generator 上旧布局方法并同步测试。

- [x] **Step 4: 运行 Sonyflake GREEN 测试**

```bash
cargo test --no-default-features --features sonyflake --verbose
```

Expected: PASS，既有等待、回拨、重启和并发行为保持。

- [x] **Step 5: 检查任务差异**

Run: `git --no-pager diff --check`

Expected: exit 0；不提交。

---

### Task 4: 增加排他到期时间与构造期 panic

**Files:**
- Create: `src/snowflake/internal/expiration_time.rs`
- Modify: `src/snowflake/internal/mod.rs`
- Modify: `src/id_error.rs`
- Modify: all three Layout、generator、builder source files
- Modify: mirrored Layout 与 builder tests

**Interfaces:**
- Produces: 每个 Layout 的 `expires_at(origin) -> Result<SystemTime, IdError>`。
- Produces: 每个 generator 的 `expires_at() -> SystemTime`。
- Produces: `IdError::ExpirationTimeOverflow { origin, time_unit, max_timestamp }`。
- Behavior: builder 在 `now >= expires_at` 时 panic。

- [x] **Step 1: 写到期边界 RED 测试**

每种算法都测试 layout 到期值、generator getter、边界前一纳秒可构造、边界及
边界后 panic，以及无法表示的到期值返回 `ExpirationTimeOverflow`。Qubit 核心
测试形状为：

```rust
let expires_at = layout
    .expires_at(epoch)
    .expect("expiration must be representable");
let wall_clock = Arc::new(ClosureWallClock::new(move || expires_at));
let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let _ = QubitSnowflakeGenerator::builder(0)
        .epoch(epoch)
        .wall_clock(wall_clock)
        .build();
}));
assert!(panic.is_err());
```

- [x] **Step 2: 运行 RED 测试**

```bash
cargo test --all-features expiration --verbose
```

Expected: FAIL，原因是 `expires_at` 和新错误 variant 尚不存在。

- [x] **Step 3: 实现 checked 到期时间计算**

```rust
pub(super) fn expiration_time(
    origin: SystemTime,
    time_unit: Duration,
    max_timestamp: u64,
) -> Result<SystemTime, IdError>;

pub(super) fn panic_if_expired(
    algorithm: &'static str,
    now: SystemTime,
    expires_at: SystemTime,
);
```

第一函数用 `time_unit.as_nanos().checked_mul(max_timestamp + 1)` 计算排他有效期，
将 u128 纳秒拆为 `Duration::new(seconds, nanos)`；任何算术、Duration 或
SystemTime 溢出都返回新错误。第二函数仅在 `now >= expires_at` 时 panic。

- [x] **Step 4: 接入三个 Layout 与 generator**

Qubit 时间单位由 precision 决定，Classic 固定 1ms，Sonyflake 使用 layout
time unit。每个 builder 只读取一次 wall clock；Sonyflake 先保留
`start_time > now` 的错误，再执行过期 panic。generator 缓存到期时间并提供
must-use inline getter；所有 build/new rustdoc 说明 panic。

- [x] **Step 5: 运行寿命 GREEN 测试**

```bash
cargo test --all-features expiration --verbose
cargo test --all-features --verbose
```

Expected: PASS；运行期越界仍返回 `TimestampOverflow`。

- [x] **Step 6: 检查任务差异**

Run: `git --no-pager diff --check`

Expected: exit 0；不提交。

---

### Task 5: 完成 must-use 与 inline 全量语义审计

**Files:**
- Modify: all scoped `src/**/*.rs`
- Test: public doctests and all existing integration tests

**Interfaces:**
- Produces: type-level must-use on domain values/builders/generators；operation-level must-use on otherwise unprotected queries and decisions。
- Produces: inline attributes matching body complexity and repository table。

- [x] **Step 1: 增加 must-use compile-fail RED doctest**

```rust
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use qubit_id::{IdMode, TimestampPrecision};
///
/// IdMode::Sequential.ordinal();
/// TimestampPrecision::Millisecond.sequence_bits();
/// ```
```

Run: `cargo test --doc --features qubit-snowflake`

Expected: FAIL，因为示例在属性加入前意外编译成功。

- [x] **Step 2: 应用 must-use 语义**

类型级覆盖 Layout、Parts、generator、builder、配置枚举、GenerationOutcome 和
UUID-like generator。函数级覆盖 trait `format_id`、primitive/String/reference
getter、decode/format 计算、内部 clock/default factory 与决策 bool。不要重复
标记返回 `Result`、`Option` 或已 must-use 类型的方法。

- [x] **Step 3: 校正 inline 与方法顺序**

getter、setter、纯委托和极薄包装使用 `#[inline(always)]`；短 constructor 和
少分支计算使用 `#[inline]`；循环、长函数、分支密集状态机不加 inline。移动
方法时携带完整 rustdoc/属性，并保持 constructor → visibility → adjacency。

- [x] **Step 4: 运行 GREEN 与静态复核**

```bash
cargo test --doc --all-features
cargo test --all-features --verbose
rg -n '#\[must_use|#\[inline' src
rg -n 'use super::\*|#\[cfg\(test\)\]' src
git --no-pager diff --check
```

Expected: tests PASS；反模式搜索无匹配；diff check exit 0。

---

### Task 6: 增加 UUID v4 对比 benchmark

**Files:**
- Modify: `Cargo.toml`
- Create: `benches/uuid_comparison/main.rs`

**Interfaces:**
- Produces: bench target `uuid_comparison`，`required-features = ["uuid"]`。
- Consumes: Mica UUID-like API 与 `uuid::Uuid::new_v4()`。

- [x] **Step 1: 运行缺失 benchmark 的 RED 命令**

```bash
cargo bench --no-default-features --features uuid \
    --bench uuid_comparison --no-run
```

Expected: FAIL，原因是 bench target 不存在。

- [x] **Step 2: 添加 target 与 dev-dependency**

```toml
[[bench]]
name = "uuid_comparison"
path = "benches/uuid_comparison/main.rs"
harness = false
required-features = ["uuid"]

[dev-dependencies]
uuid = { version = "1", features = ["v4"] }
```

- [x] **Step 3: 实现固定工作量 benchmark**

使用 `std::hint::black_box`，相同预热、样本和迭代数比较六个 case：

```text
mica_u128
uuid_v4_value
mica_hyphenated_string
uuid_v4_hyphenated_string
mica_simple_string
uuid_v4_simple_string
```

每次迭代都生成新随机值；字符串 case 包含格式化和分配。输出每个 case 的
min/median/max operations/s，不做性能阈值断言。

- [x] **Step 4: 编译并运行 GREEN benchmark**

```bash
cargo bench --no-default-features --features uuid \
    --bench uuid_comparison --no-run
cargo bench --no-default-features --features uuid \
    --bench uuid_comparison
```

Expected: exit 0；六个 case 均输出正吞吐量。

- [x] **Step 5: 检查任务差异**

Run: `git --no-pager diff --check`

Expected: exit 0；不提交。

---

### Task 7: 同步文档并完成仓库验证

**Files:**
- Modify: `README.md`
- Modify: `README.zh_CN.md`
- Modify: `src/lib.rs`
- Modify: `tests/markdown_tests/readme_examples_tests.rs`
- Modify: source/tests only for failures directly caused by Tasks 1-6

**Interfaces:**
- Produces: feature 安装、Layout/Parts、到期 panic 和 benchmark 的中英文说明。

- [x] **Step 1: 运行旧 README RED 测试**

Run: `cargo test --all-features markdown_tests --verbose`

Expected: FAIL，旧示例仍引用删除的 generator compose/extract API 或缺少新解析
结构。

- [x] **Step 2: 同步 README 与 crate docs**

文档给出默认依赖和三个可选 feature 安装片段；示例改用 Layout/Parts；明确
`expires_at` 是排他边界、构造过期 panic、无法表示边界返回配置错误。记录：

```bash
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

- [x] **Step 3: 运行 README 与 feature GREEN**

```bash
cargo test --all-features markdown_tests --verbose
cargo check --no-default-features
cargo check --no-default-features --features qubit-snowflake
cargo check --no-default-features --features classic-snowflake
cargo check --no-default-features --features sonyflake
cargo check --no-default-features --features uuid
cargo check --all-features
```

Expected: all PASS。

- [ ] **Step 4: 按规定顺序运行最终验证**

```bash
./align-ci.sh
./ci-check.sh
```

Expected: both exit 0。若且仅若 CI 报告覆盖率低于阈值，再运行：

```bash
./coverage.sh json
```

补齐真实业务分支后从 `./align-ci.sh` 重新验证。

Status: `./align-ci.sh` 已通过；`./ci-check.sh` 在 9/11 package verification
停止，因为 crates.io 尚无本地路径依赖 `qubit-clock` 0.9.0。其 1-8 阶段（含
101 个集成测试、doctest 与文档构建）均通过。

- [x] **Step 5: 最终差异审计**

```bash
git status --short
git --no-pager diff --check
git --no-pager diff --stat
```

Expected: 只有本计划内文件变化；diff check exit 0；不执行 add/commit。
