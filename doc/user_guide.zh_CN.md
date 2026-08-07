# Qubit ID 用户手册

[English user guide](user_guide.md) · [README](../README.zh_CN.md) · [API 文档](https://docs.rs/qubit-id)

## 手册目标与读者

本手册面向需要为 Rust 应用选择 ID 生成器的服务开发者，重点介绍
`qubit-id` 中的三种 Snowflake 系列生成器：`ClassicalSnowflakeGenerator`、
`SnowflakeGenerator` 和 `SonyflakeGenerator`。手册解释布局如何影响容量、排序、
存储兼容性和部署身份，并给出选型、配置、错误处理和部署边界。示例面向
`qubit-id` 0.3 和 Rust 1.94 及以上版本。

## 概念模型

每个 Snowflake 系列 ID 都组合三类信息：

```text
时间片 + 部署身份 + 时间片内的序列号
```

生成器负责分配状态。一个共享的活动生成器会串行化分配、在时间片内递增序列号，
并在时间片耗尽或可重试的时钟问题无法立即推进时等待。布局决定时间、身份和序列号
各占哪些位；Builder 决定 epoch、时钟容忍范围、重启策略以及注入的时钟能力。

下文的理论吞吐量是序列字段提供的容量上限，不是基准测试承诺。真实应用还会受到
同步、时钟采样、竞争和重试等待的影响。

## 贯穿场景：选择订单 ID 布局

假设订单服务部署在多台主机和多个进程中，需要生成能够稳定存储和传输的数值型 ID。
它需要回答以下问题：

1. 需要多少台独立主机或机器同时分配 ID？
2. 一个身份在一个时间片内需要生成多少 ID？
3. 一个 epoch 需要维持多长时间？
4. ID 是否必须小于 `i64::MAX`，以兼容有符号数据库主键？
5. 是否需要可配置布局，或降低 ID 数值与时间之间的直接关联？

先使用下表完成选型，然后为同一命名空间中每个并发运行的生成器配置独占的 host、
node 或 machine ID。

## 布局对比

| 生成器与模式 | 位布局 | 时间单位 | 身份数量 | 单身份理论吞吐量 | 理想总容量 | 时间范围 |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| 经典 Snowflake | `[reserved 1][time 41][node 10][sequence 12]` | 1 ms | 1,024 个 node | 4096/ms，约 409.6 万/s | 约 41.94 亿/s | 约 69.7 年 |
| Qubit，毫秒精度 | `[mode 1][precision 1][time 41][host 9][sequence 12]` | 1 ms | 512 个 host | 4096/ms，约 409.6 万/s | 约 20.97 亿/s | 约 69.7 年 |
| Qubit，秒精度 | `[mode 1][precision 1][time 31][host 9][sequence 22]` | 1 s | 512 个 host | 4,194,304/s，约 419.4 万/s | 约 21.47 亿/s | 约 68.1 年 |
| Sonyflake 默认配置 | `[reserved 1][time 39][sequence 8][machine 16]` | 10 ms | 65,536 个 machine | 256/10ms，约 2.56 万/s | 约 16.78 亿/s | 约 174.8 年 |

理想总容量假设每个身份都在独立并行工作，适合容量估算，不代表一个共享生成器的
实际吞吐量。Sonyflake 的可配置公式是：

```text
time_bits      = 63 - sequence_bits - machine_bits
identity_count = 2^machine_bits
throughput     = 2^sequence_bits / time_unit
time_range     = 2^time_bits × time_unit
```

### Classical Snowflake

布局固定为 41 位毫秒时间戳、10 位 node 和 12 位序列号。当兼容传统 Snowflake 风格
的 63 位布局比自定义容量规划更重要时，应优先选择它。

在同一毫秒内，ID 数值顺序是时间戳、node、sequence。它保留了熟悉的时间排序形态，
但不同 node 之间不能保证严格的全局生成先后顺序。

### Qubit Snowflake

Qubit 增加了两个自描述头部字段：

- `mode`：`Sequential` 按正常顺序存储时间戳位；`Spread` 在选定宽度内反转时间戳位。
- `precision`：`Millisecond` 使用 41 位时间戳和 12 位序列号；`Second` 使用 31 位时间戳和 22 位序列号。

选定的 mode 和 precision 会写入每个 ID，因此解码时不需要另行配置布局。`Spread`
可以降低相邻时间片 ID 的直接数值关联，但它是可逆混淆，不是加密；同一时间片内的
序列行为仍然存在。

Qubit `Spread` 始终设置第 63 位，因此生成的 ID 超过 `i64::MAX`，不能转换成有符号
数据库主键。应使用 `Id`、无符号十进制文本或 8 字节二进制传输。

### Sonyflake

Sonyflake 默认使用 39 位、10 毫秒单位的 elapsed time、8 位序列号和 16 位 machine。
默认布局更偏向机器数量和生命周期，而不是单机器突发吞吐量。Builder 可以调整 sequence
和 machine 位宽，剩余位数用于时间字段，但必须满足布局校验规则。

Sonyflake 的低位顺序是 sequence 再 machine。因此在 elapsed time 相同的情况下，数值
排序会先按 sequence，再按 machine。所有解码服务都必须知道完整的位宽和时间单位配置。

## 安装与最小配置

启用默认的 Qubit 生成器：

```toml
[dependencies]
qubit-id = "0.3"
```

通过 feature 选择其他 Snowflake 实现：

```toml
qubit-id = { version = "0.3", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["sonyflake"] }
```

根据实现不同，必需的构造身份分别称为 `host`、`node_id` 或 `machine_id`：

```rust
use qubit_id::{ClassicalSnowflakeGenerator, IdGenerationError};

fn main() -> Result<(), IdGenerationError> {
    let generator = ClassicalSnowflakeGenerator::new(7)?;
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

当需要显式设置 epoch、时钟容忍度、重启策略或注入时钟时，使用 Builder。
`ClassicalSnowflakeGenerator` 和 Qubit Snowflake 默认使用文档规定的 Qubit epoch；
Sonyflake 使用自己的默认 epoch。与既有命名空间互操作时，所有生成器和解码器必须使用
相同的 epoch 与布局配置。

## 核心工作流

1. 选择一个 feature 和一种布局。
2. 在布局的命名空间内分配独占身份。
3. 每个身份和进程边界共享一个生成器实例。
4. 需要生成器等待时使用 `generate()`；需要调用方自行安排重试时使用 `try_generate()`；
   异步流程使用 `generate_async()`。
5. 按所选布局的有符号性和序列化规则存储、传输结果。

三种 Snowflake 生成器都提供相同的主要方法：`new(...)`、`builder(...)`、`layout()`、
`epoch()`、`expires_at()`、`max_clock_skew()`、`try_generate()`、`generate()`、
`generate_async()` 和 `compose_at(time, sequence)`。

具体异步方法的外层 Future 不装箱；object-safe 的 `AsyncIdGenerator` trait 为了跨越动态
注入边界会返回装箱 Future。两种路径本身都不要求 Tokio，具体等待行为由注入的 Timer 提供。

生成器 trait 使用 `Output` 与 `Error` 泛型参数，默认值分别为 `Id` 和
`IdGenerationError`。因此，常见的同步注入边界可以简写为：

```rust
use std::sync::Arc;
use qubit_id::{BlockingIdGenerator, Id, IdGenerationError, SnowflakeGenerator};

fn create_generator() -> Result<Arc<dyn BlockingIdGenerator<Id>>, IdGenerationError> {
    Ok(Arc::new(SnowflakeGenerator::new(7)?))
}
```

自定义生成器可以显式指定两个参数，例如 `BlockingIdGenerator<String, MyError>`。

## 进阶用法

### 调整 Sonyflake 容量

如果部署需要更高的时间片内吞吐量，增加 `bits_sequence`；如果需要更多机器，增加
`bits_machine`。两种调整会消耗时间位或彼此的位空间，因此应把完整布局配置写入服务
契约。ID 进入共享命名空间后，不应在没有版本化消费者的情况下修改这些配置。

### 选择 Qubit 精度和模式

当 ID 必须携带毫秒级 elapsed time 时，使用 `TimestampPrecision::Millisecond`；当可以
接受秒级时间片且更看重更大的序列范围时，使用 `TimestampPrecision::Second`。只有在应用
确实需要降低直接数值时间关联并且能够存储无符号 64 位值时，才使用 `IdMode::Spread`。

### 测试中的确定性时钟

从同一个 `ManualMonotonicClock` 派生 WallClock 与 Timer，使测试在观察到 retry deadline
后再推进时间：

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

这样可以避免真实 sleep，并使序列耗尽、时钟回拨和到期测试可重复。

## 错误与诊断

Builder 会在返回生成器之前校验布局和生命周期。常见错误包括：

- `EpochAhead`：配置的 epoch 晚于注入的 WallClock。
- `GeneratorExpired`：`now >= expires_at`，或运行中的生成器到达排他的到期边界。
- `ClockMovedBackwards`：原始 WallClock 回拨超过 `max_clock_skew(...)`。
- `WaitFailed`：Timer 无法注册或完成重试等待。
- 布局范围错误，例如 `NodeOutOfRange`、`HostOutOfRange`、`MachineIdOutOfRange`、
  `TimestampOverflow` 或 `SequenceOverflow`。
- UUID v4 生成时操作系统无法提供随机字节，会返回 `RandomSourceFailed`。

这些情况都会返回 `IdGenerationError`，不依靠 panic 控制流程。排查命名空间冲突或到期
问题时，应记录所选布局、epoch、身份和当前时间片，同时不要把 ID 当作安全令牌。

## 排障

### 生成器返回 `GeneratorExpired`

检查配置的 epoch、所选精度或 Sonyflake 时间单位，以及 `expires_at()` 返回值。更短的
时间单位或更早的 epoch 可能让可表示生命周期更快结束。

### 重启后出现 ID 冲突

确认并发运行的进程没有复用相同的 host、node 或 machine 身份，并确认 epoch 与布局配置
属于同一个命名空间。`RestartPolicy::WaitNextSlice` 可以降低替换旧实例后复用同一时间片的
风险，但不会持久化状态，也不会协调并发的同身份进程。

### ID 无法存入有符号数据库列

当所选值保持在 `i64::MAX` 范围内时，可以使用 Classical Snowflake、Sonyflake 或
Sequential Qubit。Qubit `Spread` 设置第 63 位，必须使用无符号表示、十进制文本或二进制存储。

### 生成操作一直等待

`generate()` 可能在 WallClock 停滞、时间片内序列耗尽或可重试的时钟回拨等待时持续阻塞。
检查注入的 Timer 后端是否能独立于调用线程推进；应用必须自行安排调度和背压时使用
`try_generate()`。

## 限制与最佳实践

- 生成器不持久化分配状态，也不提供分布式身份租约。
- 同一命名空间中的每个并发生成器都需要独占身份；跨主机共享一个身份会破坏布局的唯一性假设。
- 时间排序不是安全边界。Classical、Sequential Qubit 和 Sonyflake 都暴露时间结构；
  Qubit `Spread` 只提供可逆混淆。
- 理论容量只是位字段上限。对吞吐量作运营承诺前，应基于完整服务链路进行基准测试。
- 将准确的 epoch 和布局配置写入服务契约，确保生产者、消费者和迁移工具解码同一命名空间。
- `Id` 是透明的 `u64` 值包装。启用 `serde` 后，人类可读格式使用十进制字符串，紧凑格式使用 `u64`。

## 延伸阅读

- [中文 README](../README.zh_CN.md)
- [English README](../README.md)
- [English user guide](user_guide.md)
- [API 文档](https://docs.rs/qubit-id)
