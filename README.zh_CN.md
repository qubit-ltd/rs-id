# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-id` 提供 object-safe、适合 IoC 注入的同步与异步 ID 生成器。生成器返回
领域值（`Id` 或 `Uuid`）；应用可以在存储和传输边界选择底层数值或文本表示。

## 主要特性

- `TryIdGenerator`、`IdGenerator` 与 `AsyncIdGenerator` 分别表达非阻塞、阻塞和异步分配契约。
- 每种 Snowflake 类型都实现这三个契约，并在同步和异步调用路径之间共享同一分配状态。
- `try_generate()`、`generate()` 与 `generate_async()` 共享同一分配状态；复制生成器（若支持）也共享该状态。
- Builder 接受来自 [`qubit-clock`](https://crates.io/crates/qubit-clock) 的
  `Arc<dyn WallClock>` 与 `Arc<dyn Timer>`。
- UUID 输出符合版本 4 标准，是支持 `u128` 转换和规范连字符文本的 `Uuid`。
- 默认 feature 集合只启用 `qubit-snowflake`。

## 安装

默认依赖只启用 Qubit Snowflake：

```toml
[dependencies]
qubit-id = "0.3"
```

可以独立选择其他算法：

```toml
qubit-id = { version = "0.3", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["sonyflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["uuid"] }
```

异步 ID 生成器与运行时无关。需要 Tokio Timer 时，应用应直接在自己的
`qubit-clock` 依赖上启用对应 feature。

## IoC 契约

每个生成器通过关联类型声明自己的 `Output` 和 `Error`。内置生成器使用
`IdGenerationError`，第三方生成器可以提供自己的具体错误类型。

同步 Snowflake 依赖使用
`Arc<dyn IdGenerator<Output = Id, Error = IdGenerationError>>`：

```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Output = Id, Error = IdGenerationError>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

每种 Snowflake 生成器都提供阻塞、非阻塞和异步方法。具体异步方法的外层 Future
不装箱；重试时仍可能创建 Timer 的内部 Future：

```rust
use qubit_id::{Id, IdGenerationError, SnowflakeGenerator};

async fn allocate_concrete(
    generator: &SnowflakeGenerator,
) -> Result<Id, IdGenerationError> {
    generator.generate_async().await
}
```

调用方需要自行调度重试时使用
`Arc<dyn TryIdGenerator<Output = Id, Error = IdGenerationError>>`。需要异步 object-safe
注入边界时使用 `Arc<dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>>`。动态分发
返回装箱 Future，并且不要求 Tokio：

```rust
use std::sync::Arc;
use qubit_id::{
    AsyncIdGenerator, Id, IdGenerationError, SnowflakeGenerator,
};

async fn allocate(
    generator: &dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>,
) -> Result<Id, IdGenerationError> {
    generator.generate_async().await
}

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let _injected = generator;
    Ok(())
}
```

测试 mock 只需为本地类型实现相应 trait。

## Snowflake 生成器

| Feature | 生成器类型 | 自然输出 |
| --- | --- | --- |
| `qubit-snowflake` | `SnowflakeGenerator` | `Id` |
| `classic-snowflake` | `ClassicalSnowflakeGenerator` | `Id` |
| `sonyflake` | `SonyflakeGenerator` | `Id` |

“经典 Snowflake”表示 41/10/12 位布局，并不代表存在一个通用的固定 epoch。
`ClassicalSnowflakeGenerator` 默认使用 `2018-12-02T00:00:00Z`，与 Qubit
Snowflake 相同。与使用其他时间起点的既有 ID 命名空间互操作时，应通过 Builder 的
`epoch(...)` 显式设置时间起点。

每个 Builder 只提供一个 `build()`。生成器的 `try_generate()`、`generate()` 与
`generate_async()` 共享同一分配状态；复制生成器（若支持）也共享该状态。

### 存储与传输兼容性

| 输出 | 兼容的存储与传输方式 |
| --- | --- |
| Sequential Qubit、经典 Snowflake、Sonyflake | `Id`、`u64` 或十进制文本；所选布局保持在范围内时可检查后转换为 `i64` |
| Spread Qubit | `Id`、无符号十进制文本或 8 字节二进制数据 |
| UUID v4 | `Uuid`、`u128`、16 字节二进制数据或规范 UUID 文本 |

`IdMode::Spread` 始终设置第 63 位，因此生成的 ID 必然超过 `i64::MAX`。不要将
这类 ID 强制转换为有符号数据库主键。ID 经过 JavaScript 或 JSON 边界时应使用
十进制字符串。

## 确定性时间注入

测试中应从同一个 `ManualMonotonicClock` 派生 WallClock 与 Timer。相同模式也适用于
`StdWallClock`、`StdMonotonicClock`，以及直接通过 `qubit-clock` 启用的 Tokio
Timer：

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

Manual Timer observer 可以先确认 deadline 已注册，再推进逻辑时钟，避免真实 sleep
和异步调度猜测。

Tokio Timer 会保留目标 runtime handle，因此 `generate_async()` 可以从其他 runtime 或执行
上下文轮询 timer future；目标 `Runtime` 必须保持存活并持续驱动。同步生成器会阻塞
等待，因此 timer 后端必须独立于调用线程推进；调用方需要自行控制调度时应使用
`try_generate()`，不要依赖仅由同一调用线程驱动的 Tokio current-thread runtime。

## 领域值与文本输出

Snowflake 生成器返回 `Id`。它是透明的 `u64` 值包装，并提供十进制文本显示，
调用方可以在应用边界选择需要的表示：

```rust
use qubit_id::{IdGenerationError, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let id = SnowflakeGenerator::new(7)?.generate()?;
    let numeric: u64 = id.into();
    let text = id.to_string();
    assert_eq!(text, numeric.to_string());
    Ok(())
}
```

`UuidV4Generator` 返回 `Uuid`，支持转换为 `u128`，并显示为规范的小写连字符文本。
操作系统无法提供随机字节时，UUID 生成会返回
`IdGenerationError::RandomSourceFailed`。异步应用应使用所选运行时提供的
阻塞边界包装同步调用：

```rust
use qubit_id::{IdGenerationError, UuidV4Generator};

fn main() -> Result<(), IdGenerationError> {
    let uuid = UuidV4Generator::new().generate()?;
    let numeric: u128 = uuid.into();
    let text = uuid.to_string();
    assert_ne!(numeric, 0);
    assert_eq!(text.len(), 36);
    Ok(())
}
```

## 有效期、时钟与部署身份

`expires_at()` 返回排他的到期边界。Builder 会读取注入的 WallClock。
构建器会在 `now >= expires_at` 时返回 `IdGenerationError::GeneratorExpired`，因为该配置
无法继续提供 ID。运行中的生成器随后到达相同边界时返回相同错误。

Qubit 的小幅回拨可以在 `max_clock_skew` 内等待，超过该值时返回
`IdGenerationError::ClockMovedBackwards`。经典 Snowflake 与 Sonyflake 拒绝任何回拨。
Timer 注册或阻塞适配失败会返回 `IdGenerationError::WaitFailed`，并保留原始 `TimeError`。

同一命名空间中，每个并发运行的生成器必须拥有独占的 host、node 或 machine ID。
本库不持久化分配状态，也不提供分布式租约。

`RestartPolicy::WaitNextSlice` 是所有 Snowflake Builder 的默认值，会把首次分配推迟到
后续时间片，从而降低旧实例停止后被替换时复用同一时间片的风险。
`RestartPolicy::Immediate` 供部署环境能够保证重启间隔时使用。两种策略都不能协调并发
运行的生成器，也无法防止时钟重启到更早时间片，不能替代持久化分配状态或独占的分布式身份租约。

## Feature 与基准测试

```bash
# Qubit 具体类型、动态分发与异步调用路径
cargo bench --bench qubit_snowflake_throughput

# qubit-id UUID wrapper 与直接 uuid crate 调用
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

基准测试只报告测量结果；普通测试不会断言不稳定的性能阈值。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-id](https://github.com/qubit-ltd/rs-id)
