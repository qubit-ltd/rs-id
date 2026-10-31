# Qubit ID 用户手册

[English user guide](user_guide.md) · [README](../README.zh_CN.md) · [API 文档](https://docs.rs/qubit-id)

## 手册目标与读者

本手册面向需要为 Rust 应用选择 ID 生成器的服务开发者，重点介绍
`qubit-id` 中的三种 Snowflake 系列生成器：`ClassicalSnowflakeGenerator`、
`SnowflakeGenerator` 和 `SonyflakeGenerator`。手册解释布局如何影响容量、排序、
存储兼容性和部署身份，并给出选型、配置、错误处理和部署边界。示例面向
`qubit-id` 0.7 系列（本手册核对基线为 0.7.2）和 Rust 1.94 及以上版本。

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

## 实际场景：选择订单 ID 布局

假设订单服务部署在多台主机和多个进程中，需要生成能够稳定存储和传输的数值型 ID。
它需要回答以下问题：

1. 需要多少台独立主机或机器同时分配 ID？
2. 一个身份在一个时间片内需要生成多少 ID？
3. 一个 epoch 需要维持多长时间？
4. ID 是否必须小于 `i64::MAX`，以兼容有符号数据库主键？
5. 是否需要可配置布局，或降低 ID 数值与时间之间的直接关联？

先使用下表完成选型，然后为同一命名空间中每个并发运行的生成器配置独占的 host、
node 或 machine ID。应用或部署系统必须协调不同进程的节点编号；生成器本身无法阻止
多个进程复用同一编号。

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
qubit-id = "0.7"
```

使用 `TokioTimer` 的应用应在应用自身依赖中声明 Timer 和运行时；它们不是
`qubit-id` 的生产 feature：

```toml
[dependencies]
qubit-id = "0.7"
qubit-clock = { version = "0.13", features = ["tokio"] }
tokio = { version = "1.54", features = ["macros", "rt", "time"] }
```

通过 feature 选择其他 Snowflake 实现：

```toml
qubit-id = { version = "0.7", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.7", default-features = false, features = ["sonyflake"] }
```

根据实现不同，必需的构造身份分别称为 `host`、`node_id` 或 `machine_id`：

<!-- qubit-id-example features="classic-snowflake" mode="run" -->
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
Sonyflake 使用自己的默认 epoch。epoch 不会编码为命名空间标记。例如在墙上时间
E + 10.5 秒时，采用相同 host 和布局的秒精度生成器，若 epoch 分别为 E 与 E + 1 毫秒，
都会编码 timestamp=10、sequence=0，因此可能返回相同 ID。更改 epoch 不能提供隔离；应先
评估与历史 ID 的重叠。需要隔离时，应由外部分区或存储命名空间承担。与既有命名空间互操作时，
应让生成器和解码器采用兼容的 epoch 与布局语义。

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use qubit_id::IdGenerationError;
use qubit_id::SnowflakeGenerator;

fn main() {
    assert!(matches!(
        SnowflakeGenerator::new(512),
        Err(IdGenerationError::HostOutOfRange { host: 512, max: 511 })
    ));
}
```

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

三种生成器的默认回拨容忍度不同：

| 生成器 | 默认 `max_clock_skew` |
| --- | ---: |
| Qubit Snowflake | 3 秒 |
| Classical Snowflake | 零 |
| Sonyflake | 零 |

容忍度按原始墙上时间的回退幅度判断，即使回退前后仍处于同一个编码时间片，也会检查。
容忍度为零时，只要观测到时间回退就会报错。回退没有超过已配置的容忍度时，
`try_generate()` 返回重试结果，`generate()` 同步等待，`generate_async()` 异步等待；
超过容忍度则返回 `ClockMovedBackwards`。如果部署需要明确指定策略，可以这样设置：

<!-- qubit-id-example features="classic-snowflake" mode="run" -->
```rust
use std::time::Duration;
use qubit_id::{ClassicalSnowflakeGenerator, IdGenerationError};

fn main() -> Result<(), IdGenerationError> {
    let generator = ClassicalSnowflakeGenerator::builder(7)
        .max_clock_skew(Duration::from_millis(5))
        .build()?;
    assert_eq!(generator.max_clock_skew(), Duration::from_millis(5));
    Ok(())
}
```

这个示例只检查配置值，不会模拟时钟回拨；实际回拨行为由已有生成器测试覆盖。

具体异步方法的外层 Future 不装箱；object-safe 的 `AsyncIdGenerator` trait 为了跨越动态
注入边界会返回装箱 Future。两种路径本身都不要求 Tokio，具体等待行为由注入的 Timer 提供。
`TryIdGenerator` 不等待时间推进，但分配过程可能短暂使用同步锁。`AsyncIdGenerator` 需要推进时
异步等待 Timer，分配尝试也可能使用同步锁；两者都不承诺 lock-free 或 wait-free。自定义
`WallClock` 在分配临界区中采样，`now()` 应快速返回、不执行阻塞 I/O，也不得重入同一个生成器。

生成器 trait 使用 `Output` 与 `Error` 泛型参数，默认值分别为 `Id` 和
`IdGenerationError`。因此，常见的同步注入边界可以简写为：

<!-- qubit-id-example features="qubit-snowflake" mode="check" -->
```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn create_generator() -> Result<Arc<dyn IdGenerator<Id>>, IdGenerationError> {
    Ok(Arc::new(SnowflakeGenerator::new(7)?))
}
```

自定义生成器可以显式指定两个参数，例如 `IdGenerator<String, MyError>`。

### 使用 Tokio 异步生成

`qubit-id` 与运行时无关。示例在 Tokio runtime 内创建 `TokioTimer`；
`WaitNextSlice` 表示生成操作确实可能等待下一时间片。五秒 timeout 是应用设置的总等待预算。
`generate()` 是阻塞 API；异步流程应使用 `generate_async()`。

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use qubit_clock::TokioTimer;
use qubit_id::{AsyncIdGenerator, Id, RestartPolicy, SnowflakeGenerator, TimestampPrecision};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(TokioTimer::try_current()?);
    let generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(
        SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .timer(timer)
            .build()?,
    );
    let id = tokio::time::timeout(Duration::from_secs(5), generator.generate_async()).await??;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

### 使用 `try_generate` 自行安排重试

只有生成器返回 `RetryAfter` 时才通过 `Timer` 等待；其他生成错误、Timer 错误和 timeout
错误都会传播给调用方。timeout 是应用设置的总等待预算。

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use qubit_clock::{Timer, TokioTimer};
use qubit_id::{GenerationAttempt, Id, RestartPolicy, SnowflakeGenerator, TimestampPrecision};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(TokioTimer::try_current()?);
    let generator = SnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .timer(timer.clone())
        .build()?;
    let id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match generator.try_generate()? {
                GenerationAttempt::Generated(id) => {
                    return Ok::<Id, Box<dyn std::error::Error>>(id);
                }
                GenerationAttempt::RetryAfter { delay } => {
                    timer.after(delay)?.await?;
                }
            }
        }
    }).await??;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

## 进阶用法

### 调整 Sonyflake 容量

如果部署需要更高的时间片内吞吐量，增加 `bits_sequence`；如果需要更多机器，增加
`bits_machine`。两种调整会消耗时间位或彼此的位空间，因此应把完整布局配置写入服务
契约，并制定迁移方案。对消费者进行版本化可以保持解码兼容，但不会让新旧数值 ID 自动
互不重叠。使用相同 epoch 和时间单位、machine ID 7、sequence 0 时，旧布局
`(bits_sequence=8, bits_machine=16)` 在 elapsed time 为 1 时，以及新布局 `(8, 15)`
在 elapsed time 为 2 时，都会生成 `16777223`。因此，即使所有消费者都能正确解码各自的
布局，先停掉旧生成器再启动新生成器也可能复用历史数值 ID。更换 epoch 不会自动隔离；
如果数据库唯一键只是裸 ID，仅在 decoder 配置中记录版本也无法防止冲突。共享裸 ID
命名空间应保持现有配置；迁移时应使用独立命名空间、把外部版本纳入存储键，或证明新旧
分配范围不相交。

使用 `SonyflakeLayout::new` 或 Builder 时，`bits_sequence`、`bits_machine` 设为零表示
采用默认位宽，分别是 8 位和 16 位。显式指定的非零位宽都必须小于 31；归一化后的
sequence 与 machine 位宽合计后，至少要为时间字段留下 32 位，即
`bits_time = 63 - bits_sequence - bits_machine`。时间单位不能短于 1 毫秒，machine ID
也必须能由 machine 字段表示。即使字段宽度合法，所选 epoch 与时间单位下的生命周期也
可能无法表示，或生成器已经到期；Builder 返回生成器前会校验这两项。结构化错误见
[`IdGenerationError`](../src/id_generation_error.rs)。

下面的可运行示例配置新的 Sonyflake 命名空间，并检查布局、编码字段和到期边界：

<!-- qubit-id-example features="sonyflake" mode="run" -->
```rust
use std::time::Duration;
use qubit_id::{IdGenerationError, SonyflakeGenerator, SonyflakeLayout};

fn main() -> Result<(), IdGenerationError> {
    let generator = SonyflakeGenerator::builder(7)
        .bits_sequence(12)
        .bits_machine(12)
        .time_unit(Duration::from_millis(10))
        .build()?;
    let layout = generator.layout();
    assert_eq!(layout.bits_time(), 39);
    assert_eq!(layout.bits_sequence(), 12);
    assert_eq!(layout.bits_machine(), 12);
    assert_eq!(layout.time_unit(), Duration::from_millis(10));
    assert_eq!(layout.max_sequence(), 4095);
    assert_eq!(layout.max_machine_id(), 4095);
    let id = layout.compose(42, 3)?;
    let parts = layout.decode(id);
    assert_eq!(parts.elapsed_time(), 42);
    assert_eq!(parts.sequence(), 3);
    assert_eq!(parts.machine_id(), 7);
    assert_eq!(generator.expires_at(), layout.expires_at(generator.epoch())?);
    assert!(generator.expires_at() > generator.epoch());
    assert!(matches!(
        SonyflakeLayout::new(7, 16, 16, Duration::from_millis(10)),
        Err(IdGenerationError::InvalidBitLength { name: "time", .. })
    ));
    Ok(())
}
```

12 位 sequence 和 12 位 machine 各自可表示 4,096 个值。39 位时间字段配合 10 毫秒单位，
可表示约 174.8 年；这些容量不代表实际生成吞吐量。`layout.compose(...)` 只按给定字段
组合 ID，供本例检查使用，不会分配或预留唯一 ID。此配置属于新的命名空间，不能直接替换
已有共享命名空间的配置；迁移时应按上文选择隔离方案，避免复用数值 ID。

### 选择 Qubit 精度和模式

当 ID 必须携带毫秒级 elapsed time 时，使用 `TimestampPrecision::Millisecond`；当可以
接受秒级时间片且更看重更大的序列范围时，使用 `TimestampPrecision::Second`。只有在应用
确实需要降低直接数值时间关联并且能够存储无符号 64 位值时，才使用 `IdMode::Spread`。

### 测试中的确定性时钟

下面的独立程序从同一个 `ManualMonotonicClock` 派生 WallClock 与 Timer。observer 会先确认
retry deadline 已注册，然后 `advance_to_next_deadline()` 才推进逻辑时间。程序成功时，所有
断言通过并正常退出。timeout 只用于防止示例挂起，不负责推进逻辑时间。这里使用
`WaitNextSlice` 是为了刻意触发一次等待；它不提供持久化或跨进程唯一性保证。

请使用以下精确依赖；无需启用时钟库的 `test-util` feature，也无需在 `qubit-clock` 上启用
Tokio timer feature：

```toml
[dependencies]
qubit-id = "0.7"
qubit-clock = "0.13"
tokio = { version = "1.54", features = ["macros", "rt", "time"] }
```

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_id::RestartPolicy;
use qubit_id::SnowflakeGenerator;
use qubit_id::TimestampPrecision;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let initial_time = epoch + Duration::from_secs(10);
    let clock = ManualMonotonicClock::new_shared();
    let generator = Arc::new(
        SnowflakeGenerator::builder(7)
            .epoch(epoch)
            .precision(TimestampPrecision::Millisecond)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(clock.new_wall_clock(initial_time))
            .timer(clock.new_timer())
            .build()?,
    );
    let worker_generator = Arc::clone(&generator);
    let task = tokio::spawn(async move { worker_generator.generate_async().await });
    let observed = tokio::time::timeout(
        Duration::from_secs(5),
        clock.wait_for_next_deadline_async(),
    )
    .await?;
    assert_eq!(observed.elapsed_since_origin(), Duration::from_millis(1));
    assert!(!task.is_finished());
    let reached = clock.advance_to_next_deadline().expect("registered deadline");
    assert_eq!(reached, observed);
    let id = tokio::time::timeout(Duration::from_secs(5), task).await???;
    let expected = generator.compose_at(initial_time + Duration::from_millis(1), 0)?;
    assert_eq!(id, expected);
    Ok(())
}
```

一毫秒断言验证等待时长；将返回 ID 与新时间片上 sequence 为 0 的 ID 比较，则验证了返回结果。

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

阻塞和异步 API 会自动等待可重试的时间片推进。若时钟回拨超过容忍值，应保留原生成器，
让时钟追上其保留的高水位；使用相同身份重建生成器不能安全地清除该状态。`WaitFailed`
需要根据注入 Timer 的后端采取恢复措施。取消属于应用控制流，应与生成错误分别处理。
取消未完成的异步等待会保留生成器的共享分配状态；取消也不能撤销已经返回给调用方的 ID。
`EpochAhead` 和 `GeneratorExpired` 需要修正配置或部署到有效的新命名空间，它们不是临时重试信号。

## 排障

### 生成器返回 `GeneratorExpired`

检查配置的 epoch、所选精度或 Sonyflake 时间单位，以及 `expires_at()` 返回值。更短的
时间单位或更早的 epoch 可能让可表示生命周期更快结束。

### 重启后出现 ID 冲突

确认并发运行的进程没有复用相同的 host、node 或 machine 身份，并确认 epoch 与布局配置
属于同一个命名空间。`RestartPolicy::WaitNextSlice` 可以降低替换旧实例后复用同一时间片的
风险，但不会持久化状态，也不会协调并发的同身份进程。该策略只有在替代实例首次观测的
时间片不早于前驱最后分配的时间片时，才能降低顺序替换造成的同片复用风险。首次分配没有
耗时上界承诺，时钟回退和调度都可能延长等待。

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
- `Id::to_padded_decimal()` 返回固定 20 位的十进制字符串，可用于需要让词典序与 `u64`
  数值顺序一致的持久化文本键。普通 `Display` 输出仍不补零。
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
