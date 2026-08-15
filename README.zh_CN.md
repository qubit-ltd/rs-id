# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-id` 为需要数值型 ID 边界的应用提供 object-safe、适合 IoC 注入的同步与异步
ID 生成器。它包含三种 Snowflake 系列布局和 UUID v4 生成器，提供类型化的 `Id`、
结构化错误、可注入的确定性时钟，以及阻塞、非阻塞和异步分配 API。

## 选型依据

假设订单服务需要在多个进程中生成 ID，同时要在有符号 64 位数据库主键兼容性、机器
命名空间规模和布局可配置性之间做取舍。可以先根据下表选择生成器，再阅读[中文用户手册](doc/user_guide.zh_CN.md)，了解完整的部署检查项和配置权衡。

| 生成器 | 布局与时间单位 | 节点数 | 单节点理论吞吐量 | 时间范围 | 适用场景 |
| --- | --- | ---: | ---: | ---: | --- |
| `ClassicalSnowflakeGenerator` | `1 + 41 ms + 10 node + 12 sequence` | 1,024 | 4096/ms，约 409.6 万/s | 约 69.7 年 | 需要最简单、传统、兼容有符号 63 位整数的布局。 |
| `SnowflakeGenerator` | `1 mode + 1 precision + 41 ms + 9 host + 12 sequence` | 512 | 4096/ms，约 409.6 万/s | 约 69.7 年 | 需要带精度信息的 Qubit 布局和毫秒精度。 |
| `SnowflakeGenerator` | `1 mode + 1 precision + 31 s + 9 host + 22 sequence` | 512 | 4,194,304/s，约 419.4 万/s | 约 68.1 年 | 可以使用秒精度，并需要较大的单秒突发容量。 |
| `SonyflakeGenerator`（默认布局） | `1 + 39 time + 8 sequence + 16 machine`，10 ms/单位 | 65,536 | 256/10ms，约 2.56 万/s | 约 174.8 年 | 需要更大的机器命名空间、更长寿命或可配置位宽。 |

表中的吞吐量是位字段容量上限，不是基准测试承诺。理想情况下，多节点总上限等于
单节点吞吐量乘以节点数；实际吞吐量还受锁竞争、时钟推进和重试等待影响。Sonyflake
可以通过调整字段位宽，在时间范围、序列容量和机器数量之间做取舍。Qubit 的
`Spread` 模式与所选精度拥有相同容量，但会反转时间戳位以降低数值 ID 与时间的直接关联；
它是可逆混淆，不是加密。

## 主要特性

- `IdGenerator`、`TryIdGenerator` 与 `AsyncIdGenerator` 分别提供阻塞、非阻塞和异步分配能力；
  三者都通过泛型参数提供 `Output` 与 `Error`，默认值为 `Id` 和 `IdGenerationError`。
- 每种 Snowflake 类型都实现这三种能力，并在同步与异步调用路径之间共享同一分配状态。
- Builder 接受来自 [`qubit-clock`](https://crates.io/crates/qubit-clock) 的
  `Arc<dyn WallClock>` 与 `Arc<dyn Timer>`，使时钟回拨、序列耗尽和重试行为可以在测试中确定性复现。
- UUID 输出使用符合版本 4 标准的 `uuid::Uuid`。
- 默认 feature 集合只启用 `qubit-snowflake`。

## 安装

默认依赖只启用 Qubit Snowflake：

```toml
[dependencies]
qubit-id = "0.4"
```

可以独立选择其他算法：

```toml
qubit-id = { version = "0.4", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.4", default-features = false, features = ["sonyflake"] }
qubit-id = { version = "0.4", default-features = false, features = ["uuid"] }
qubit-id = { version = "0.4", default-features = false, features = ["serde"] }

# 使用 UUID 的应用应直接依赖上游类型。
uuid = { version = "1", features = ["v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

异步 ID 生成器与运行时无关。需要 Tokio Timer 时，应用应直接在自己的
`qubit-clock` 依赖上启用对应 feature。

## 快速开始

下面的服务内生成器使用 host `7`，返回类型化的 `Id`：
`SnowflakeGenerator::new` 默认使用 Sequential 模式和秒精度；需要毫秒级时间戳
时，应通过 Builder 显式选择 `TimestampPrecision::Millisecond`。

```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Id>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

每种 Snowflake 生成器都提供 `try_generate()`、`generate()` 和
`generate_async()`。具体异步方法的外层 Future 不装箱；重试时仍可能创建 Timer 的
内部 Future。需要异步 object-safe 注入边界时使用
`Arc<dyn AsyncIdGenerator<Id>>`。

## Snowflake 生成器

| Feature | 生成器类型 | 布局类型 |
| --- | --- | --- |
| `qubit-snowflake` | `SnowflakeGenerator` | `SnowflakeLayout` |
| `classic-snowflake` | `ClassicalSnowflakeGenerator` | `ClassicalSnowflakeLayout` |
| `sonyflake` | `SonyflakeGenerator` | `SonyflakeLayout` |

三种布局的主要边界都使用类型化的 `Id`。只有在与协议、数据库位模式或其他 `u64` API
互操作时，才使用 raw 方法：

```rust
use qubit_id::ClassicalSnowflakeLayout;

fn main() -> Result<(), qubit_id::IdGenerationError> {
    let layout = ClassicalSnowflakeLayout::new(7)?;
    let id = layout.compose(42, 3)?;
    let parts = ClassicalSnowflakeLayout::decode(id);
    let raw = layout.compose_raw(42, 3)?;
    let raw_parts = ClassicalSnowflakeLayout::decode_raw(raw);
    assert_eq!(parts, raw_parts);
    Ok(())
}
```

“经典 Snowflake”表示 41/10/12 位布局，并不代表存在一个通用的固定 epoch。
`ClassicalSnowflakeGenerator` 默认使用 `2018-12-02T00:00:00Z`，与 Qubit
Snowflake 相同。与使用其他时间起点的既有 ID 命名空间互操作时，应通过 Builder 的
`epoch(...)` 显式设置时间起点。

详细的布局对比、选择规则、错误处理和部署限制见[中文用户手册](doc/user_guide.zh_CN.md)
和[English user guide](doc/user_guide.md)。

## IoC 契约与运行时行为

每个生成器通过泛型参数声明 `Output` 与 `Error`，默认值分别为 `Id` 和
`IdGenerationError`。第三方生成器可以显式指定自己的错误类型。需要调用方透明等待时选择
`IdGenerator`；需要调用方自行安排重试时选择 `TryIdGenerator`；需要异步 object-safe
边界时选择 `AsyncIdGenerator`。

同步 object-safe 依赖可以使用
`Arc<dyn IdGenerator<Id>>`；UUID 依赖可以使用
`Arc<dyn IdGenerator<Uuid>>`。

所有 Builder 都提供 `epoch(...)`、`max_clock_skew(...)` 与 `restart_policy(...)`。
`IdGenerator` 通过阻塞的 `generate()` 提供同步分配能力。
`RestartPolicy::Immediate` 是所有 Snowflake Builder 的默认值。
`try_generate()`、`generate()` 与 `generate_async()` 共享同一分配状态；复制生成器（若支持）也共享该状态。

三种 Snowflake 生成器还提供 `layout()`、`epoch()`、`expires_at()`、
`max_clock_skew()` 和 `compose_at(time, sequence)`。

`expires_at()` 返回排他的到期边界。Builder 会在 epoch 晚于注入的 WallClock 时返回
`IdGenerationError::EpochAhead`。构建器会在 `now >= expires_at` 时返回 `IdGenerationError::GeneratorExpired`。
时钟回拨超过容忍范围时返回
`IdGenerationError::ClockMovedBackwards`；Timer 失败时返回
`IdGenerationError::WaitFailed`。这些都是结构化错误，不依靠 panic 控制流程。操作系统无法
提供随机字节时，UUID 生成会返回 `IdGenerationError::RandomSourceFailed`。

同一命名空间中，每个并发运行的生成器必须拥有独占的 host、node 或 machine ID。本库不持久化
分配状态，也不提供分布式租约。

## 存储与传输

| 生成器模式 | 推荐表示 |
| --- | --- |
| Sequential Qubit、经典 Snowflake、Sonyflake | `Id`、`u64` 或十进制文本；所选布局保持在范围内时可检查后转换为 `i64` |
| Spread Qubit | `Id`、无符号十进制文本或 8 字节二进制数据 |
| UUID v4 | `uuid::Uuid`、16 字节二进制数据或规范 UUID 文本 |

`IdMode::Spread` 始终设置第 63 位，因此生成的 ID 必然超过 `i64::MAX`。不要将这类
ID 强制转换为有符号数据库主键。ID 经过 JavaScript 或 JSON 边界时应使用十进制字符串。
启用可选 `serde` feature 后，人类可读的 `Id` 使用十进制字符串，紧凑格式使用 `u64`。

## 确定性时间注入

测试中应从同一个 `ManualMonotonicClock` 派生 WallClock 与 Timer：

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

Manual Timer observer 可以先确认 deadline 已注册，再推进逻辑时钟，避免真实 sleep 和异步调度猜测。

## UUID v4

`UuidV4Generator` 返回 `uuid::Uuid`。应用应直接从 `uuid` 依赖导入 `Uuid`，使用其解析、
格式化、版本和字节 API。异步应用应使用所选运行时提供的阻塞边界包装同步调用：

```rust
use std::sync::Arc;
use qubit_id::{IdGenerationError, IdGenerator, UuidV4Generator};
use uuid::Uuid;

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Uuid>> =
        Arc::new(UuidV4Generator::new());
    let uuid = generator.generate()?;
    assert_eq!(uuid.to_string().len(), 36);
    Ok(())
}
```

## Feature 与基准测试

```bash
# Qubit 具体类型、动态分发与异步调用路径
cargo bench --bench qubit_snowflake_throughput

# qubit-id UUID wrapper 与直接 uuid crate 调用
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

## 延伸阅读

- [中文用户手册](doc/user_guide.zh_CN.md)
- [English user guide](doc/user_guide.md)
- [English README](README.md)
- [API 文档](https://docs.rs/qubit-id)

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
