# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-id` 提供 object-safe、适合 IoC 注入的同步与异步 ID 生成器。应用可以按
自然输出类型（`u64`、`u128` 或 `String`）选择实现，也能在测试中注入本地 mock。

## 主要特性

- `IdGenerator<T, E = IdError>` 与
  `AsyncIdGenerator<T, E = IdError>` 支持生成器自有错误类型，并使用 `&self`；
  状态变化由实现内部同步。
- Qubit Snowflake、经典 Snowflake 与 Sonyflake 共享一个分配核心，同时使用独立的
  阻塞和异步等待驱动器。
- Builder 接受来自 [`qubit-clock`](https://crates.io/crates/qubit-clock) 的
  `Arc<dyn WallClock>` 与 `Arc<dyn Timer>`。
- UUID 输出符合版本 4 标准，可以选择 `u128` 或规范的连字符字符串。
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

错误类型参数默认为 `IdError`，所以内置生成器继续使用简短的
`IdGenerator<T>` 写法。第三方实现可以通过 `IdGenerator<T, E>` 保留自己的
具体错误类型。

同步数值依赖使用 `Arc<dyn IdGenerator<u64>>`：

```rust
use std::sync::Arc;
use qubit_id::{IdError, IdGenerator, QubitSnowflakeGenerator};

fn main() -> Result<(), IdError> {
    let generator: Arc<dyn IdGenerator<u64>> =
        Arc::new(QubitSnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id, 0);
    Ok(())
}
```

具体异步生成器提供不装箱的 inherent Future：

```rust
use qubit_id::{AsyncQubitSnowflakeGenerator, IdError};

async fn allocate_concrete(
    generator: &AsyncQubitSnowflakeGenerator,
) -> Result<u64, IdError> {
    generator.generate_async().await
}
```

需要 object-safe 注入边界时使用 `Arc<dyn AsyncIdGenerator<u64>>`。动态分发
返回装箱 Future，并且不要求 Tokio：

```rust
use std::sync::Arc;
use qubit_id::{
    AsyncIdGenerator, AsyncQubitSnowflakeGenerator, IdError,
};

async fn allocate(
    generator: &dyn AsyncIdGenerator<u64>,
) -> Result<u64, IdError> {
    generator.generate_async().await
}

fn main() -> Result<(), IdError> {
    let generator: Arc<dyn AsyncIdGenerator<u64>> =
        Arc::new(AsyncQubitSnowflakeGenerator::new(7)?);
    let _injected = generator;
    Ok(())
}
```

测试 mock 只需为本地类型实现相应 trait。

## Snowflake 生成器

| Feature | 同步类型 | 异步类型 | 自然输出 |
| --- | --- | --- | --- |
| `qubit-snowflake` | `QubitSnowflakeGenerator` | `AsyncQubitSnowflakeGenerator` | `u64` |
| `classic-snowflake` | `SnowflakeGenerator` | `AsyncSnowflakeGenerator` | `u64` |
| `sonyflake` | `SonyflakeGenerator` | `AsyncSonyflakeGenerator` | `u64` |

“经典 Snowflake”表示 41/10/12 位布局，并不代表存在一个通用的固定 epoch。
`SnowflakeGenerator` 默认使用 `2018-12-02T00:00:00Z`，与 Qubit Snowflake
相同。与使用其他时间起点的既有 ID 命名空间互操作时，应通过 Builder 的
`epoch(...)` 显式设置时间起点。

每个 Builder 都提供 `build()` 与 `build_async()`；两条路径复用相同的布局、
epoch/start time、restart policy、WallClock 与 Timer 配置。

每次成功调用 `build()` 或 `build_async()` 都会创建独立的分配状态。即使配置完全
相同，两个生成器也不会协调序列，包括一个同步生成器和一个异步生成器的组合。

### 存储与传输兼容性

| 输出 | 兼容的存储与传输方式 |
| --- | --- |
| Sequential Qubit、经典 Snowflake、Sonyflake | `u64`；所选布局保持在范围内时可检查后转换为 `i64` |
| Spread Qubit | `u64`、无符号十进制文本或 8 字节二进制数据 |
| UUID v4 | `u128`、16 字节二进制数据或规范 UUID 文本 |

`IdMode::Spread` 始终设置第 63 位，因此生成的 ID 必然超过 `i64::MAX`。不要将
这类 ID 强制转换为有符号数据库主键。ID 经过 JavaScript 或 JSON 边界时应使用
十进制字符串。

## 确定性时间注入

测试中应从同一个 `ManualMonotonicClock` 派生 WallClock 与 Timer。相同模式也适用于
`StdWallClock`、`StdMonotonicClock`，以及直接通过 `qubit-clock` 启用的 Tokio
Timer：

```text
let clock = ManualMonotonicClock::new_shared();
let generator = QubitSnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build_async()?;
```

Manual Timer observer 可以先确认 deadline 已注册，再推进逻辑时钟，避免真实 sleep
和异步调度猜测。

Tokio Timer 会保留目标 runtime handle，因此异步生成器可以从其他 runtime 或执行
上下文轮询 timer future；目标 `Runtime` 必须保持存活并持续驱动。同步生成器会阻塞
等待，因此 timer 后端必须独立于调用线程推进；不要依赖仅由同一调用线程驱动的
Tokio current-thread runtime。

## 字符串与 UUID 输出

IoC 边界需要十进制文本时，可以包装任意同步或异步 Snowflake `u64` 生成器：

```rust
use std::sync::Arc;
use qubit_id::{
    IdError, IdGenerator, QubitSnowflakeGenerator,
    SnowflakeStringGenerator,
};

fn main() -> Result<(), IdError> {
    let numeric = QubitSnowflakeGenerator::new(7)?;
    let generator: Arc<dyn IdGenerator<String>> =
        Arc::new(SnowflakeStringGenerator::new(numeric));
    let value = generator.generate()?;
    assert!(value.parse::<u64>().is_ok());
    Ok(())
}
```

`UuidV4Generator` 返回 `u128`，`UuidV4StringGenerator` 返回规范的连字符文本；
两者都只实现同步契约。操作系统无法提供随机字节时，
UUID 生成会返回 `IdError::RandomSourceFailed`。异步应用应使用所选运行时提供的
阻塞边界包装同步调用：

```rust
use qubit_id::{
    IdError, IdGenerator, UuidV4Generator, UuidV4StringGenerator,
};

fn main() -> Result<(), IdError> {
    let numeric = UuidV4Generator::new().generate()?;
    let text = UuidV4StringGenerator::new().generate()?;
    assert_ne!(numeric, 0);
    assert_eq!(text.len(), 36);
    Ok(())
}
```

## 有效期、时钟与部署身份

`expires_at()` 返回排他的到期边界。Builder 会读取注入的 WallClock。
构建器会在 `now >= expires_at` 时返回 `IdError::GeneratorExpired`，因为该配置
无法继续提供 ID。运行中的生成器随后到达相同边界时返回相同错误。

Qubit 的小幅回拨可以在 `max_clock_skew` 内等待，超过该值时返回
`IdError::ClockMovedBackwards`。经典 Snowflake 与 Sonyflake 拒绝任何回拨。
Timer 注册或阻塞适配失败会返回 `IdError::WaitFailed`，并保留原始 `TimeError`。

同一命名空间中，每个并发运行的生成器必须拥有独占的 host、node 或 machine ID。
本库不持久化分配状态，也不提供分布式租约。

`RestartPolicy::Immediate` 会在当前时间片开始分配，不提供重启隔离。
`RestartPolicy::WaitNextSlice` 会把首次分配推迟到后续时间片，从而降低旧实例停止后
被替换时复用同一时间片的风险；它不能协调并发运行的生成器，也无法防止时钟重启到
更早时间片。两种策略都不能替代持久化分配状态或独占的分布式身份租约。

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
