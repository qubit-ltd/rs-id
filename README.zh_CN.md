# Qubit ID (`rs-id`)

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Docs.rs](https://docs.rs/qubit-id/badge.svg)](https://docs.rs/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

文档：[API Reference](https://docs.rs/qubit-id)

`qubit-id` 为 Rust 服务提供 ID 生成工具。

它包含一个统一的 `IdGenerator` trait，其 ID 和错误使用关联类型表示；同时提供数据库友好的 Snowflake ID、Sonyflake 风格 ID，以及快速随机 UUID-like 字符串。

## 适用场景

当你需要以下能力时，可以使用 `qubit-id`：

- 带固定高位 mode 和 precision 头部的 Qubit Snowflake ID
- 经典 Snowflake 布局的 64 位数字 ID
- Sonyflake 风格 ID，在较小序列位宽下换取更长可用时间和更大机器号空间
- 快速 UUID-like 随机字符串
- 通过同一个 trait 同时获得强类型 ID 和字符串表示

## 安装

```toml
[dependencies]
qubit-id = "0.3"
```

## 快速开始

```rust
use qubit_id::{
    GenerationOutcome, IdGenerator, MicaUuidLikeGenerator,
    QubitSnowflakeGenerator,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    match snowflake.try_next_id()? {
        GenerationOutcome::Generated(id) => println!("{id}"),
        GenerationOutcome::RetryAfter(duration) => {
            println!("请在 {duration:?} 后重试");
        }
    }

    let uuid_like = MicaUuidLikeGenerator::new();
    let uuid_like_value: u128 = uuid_like.next_id()?;
    let uuid_like_text = uuid_like.next_string()?;

    println!("{id} {id_text} {uuid_like_value} {uuid_like_text}");
    Ok(())
}
```

## 核心 API

| 类型 | 作用 |
| --- | --- |
| `IdGenerator` | 使用关联 ID 和错误类型的统一生成与字符串格式化 trait。 |
| `GenerationOutcome<T>` | 一次不休眠生成尝试的结果：生成 ID 或给出重试时长。 |
| `RestartPolicy` | 控制新的 Snowflake 生成器立即分配，还是等待下一个时间片。 |
| `QubitSnowflakeGenerator` | Qubit 固定头部 Snowflake 生成器。 |
| `QubitSnowflakeGeneratorBuilder` | 配置 Qubit Snowflake 生成器。 |
| `QubitSnowflakeLayout` | 组合 Qubit Snowflake ID，并根据固定头部解析任意 Qubit 布局。 |
| `QubitSnowflakeParts` | `QubitSnowflakeLayout::decode` 返回的字段。 |
| `SnowflakeGenerator` | 经典 41 位时间、10 位节点、12 位序列 Snowflake 生成器。 |
| `SnowflakeGeneratorBuilder` | 配置经典 Snowflake 生成器。 |
| `SonyflakeGenerator` | 支持配置序列位和机器位的 Sonyflake 风格生成器。 |
| `SonyflakeGeneratorBuilder` | 配置 Sonyflake 风格生成器。 |
| `MicaUuidLikeGenerator` | Mica 风格随机 128 位 UUID-like 生成器。 |
| `fast_uuid_like` | 生成小写标准形态 UUID-like 字符串。 |
| `fast_simple_uuid_like` | 生成小写 32 位十六进制 UUID-like 字符串。 |

## 唯一性与部署要求

三种 Snowflake 系列生成器都是线程安全的。同一个存活且共享的 generator
实例上，成功的 `next_id` 和 `next_string` 调用不会生成重复 ID。一个进程
内，每个 ID 命名空间应该共享一个 generator 实例，不应为每个线程或请求
分别创建实例。

跨进程、跨服务器时，所有可能向同一命名空间生成 ID 的并行 generator
实例必须独占各自的身份编号：

- `QubitSnowflakeGenerator` 使用 `host`
- `SnowflakeGenerator` 使用 `node_id`
- `SonyflakeGenerator` 使用 `machine_id`

本 crate 不负责分配或协调这些编号。不同的 epoch、start time 或位布局也
可能产生相同的数值，因此部署配置本身也是 ID 命名空间的一部分。

默认重启策略是 `RestartPolicy::Immediate`：首次调用不会等待，而是在当前
逻辑时间片立即分配序列号零。分配状态不会持久化。状态丢失或实例替换后，
只有以下三个条件同时成立才会重复 ID：实例使用相同的有效身份、布局和参考
时间；它们在同一逻辑时间片内分配；并且分配的序列号范围重叠。有效身份分别
是 `host`、`node_id` 或 `machine_id`，参考时间是 epoch 或 Sonyflake start
time。

`RestartPolicy::WaitNextSlice` 会记录新生成器首次观察到的时间片，并等到后续
时间片再开始分配。它只保护旧实例已停止、新实例随后启动的顺序替换场景，
不能协调使用相同有效身份的并发实例：这些实例可能一起越过等待边界，并分配
重叠的序列号范围。并发复用同一身份仍然需要外部独占 lease 或等价协调。
本 crate 不提供持久化分配状态，也不提供跨进程协调。

`try_next_id()` 和 `try_next_string()` 只执行一次分配尝试，绝不会为等待时钟
前进而休眠，也不会调用配置的 sleeper；使用同步状态的生成器仍可能短暂等待
内部 mutex。`next_id()` 和 `next_string()` 会把重试结果转换为阻塞等待。序列
耗尽或 Qubit 出现可容忍回拨时，正常情况下会等待大约一个配置时间单位；但
如果墙上时钟停止，或者
注入的 sleeper 没有推动墙上时钟前进，它们可能无限期等待。Qubit Snowflake
会在配置的回拨容忍范围内等待重试，超过范围时报错；经典 Snowflake 和
Sonyflake 对任何已观测到的回拨都会立即报错。

`compose`、`generate_at` 和 `decode` 都是无状态转换，不提供唯一性保证。
对任意 `u64` 解码只会提取字段，不会鉴别或验证该值。
`MicaUuidLikeGenerator` 使用 128 位随机数，因此只提供概率意义上的唯一性，
理论上仍可能碰撞。

## Generator 使用示例

### QubitSnowflakeGenerator

需要使用 Qubit 固定头部 Snowflake 布局时，使用
`QubitSnowflakeGenerator`。默认构造函数使用顺序模式、秒精度和默认
Qubit epoch。

```rust
use qubit_id::{
    IdGenerator, QubitSnowflakeGenerator, QubitSnowflakeLayout,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 参数是编码到生成 ID 中的 9 位 host ID。
    // 取值范围必须是 0..=511。
    let generator = QubitSnowflakeGenerator::new(42)?;

    let id = generator.next_id()?;
    let id_text = generator.next_string()?;

    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.host(), 42);

    println!("{id} {id_text}");
    Ok(())
}
```

如果需要打散模式或毫秒精度，可以显式配置 Qubit 布局。下面的示例还选择了
`WaitNextSlice`；省略该 setter 时使用默认的 `Immediate` 策略。

```rust
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{
    IdGenerator, IdMode, QubitSnowflakeGenerator, QubitSnowflakeLayout,
    RestartPolicy, TimestampPrecision,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = QubitSnowflakeGenerator::builder(7)
        .mode(IdMode::Spread)
        .precision(TimestampPrecision::Millisecond)
        .epoch(UNIX_EPOCH + Duration::from_millis(1_543_708_800_000))
        .restart_policy(RestartPolicy::WaitNextSlice)
        .build()?;

    let id = generator.next_id()?;
    let parts = QubitSnowflakeLayout::decode(id);

    assert_eq!(parts.mode(), IdMode::Spread);
    assert_eq!(parts.precision(), TimestampPrecision::Millisecond);
    assert_eq!(parts.host(), 7);

    Ok(())
}
```

### SnowflakeGenerator

需要经典的 41 位毫秒时间、10 位节点和 12 位序列布局时，使用
`SnowflakeGenerator`。

```rust
use qubit_id::{IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SnowflakeGenerator::new(3)?;

    let id = generator.next_id()?;

    assert_eq!(generator.extract_node_id(id), 3);
    println!("{id}");

    Ok(())
}
```

也可以用已知字段手动组合并解析确定性的 ID。

```rust
use qubit_id::SnowflakeGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SnowflakeGenerator::new(3)?;
    let id = generator.compose(1_000, 5)?;

    assert_eq!(generator.extract_timestamp(id), 1_000);
    assert_eq!(generator.extract_node_id(id), 3);
    assert_eq!(generator.extract_sequence(id), 5);

    Ok(())
}
```

### SonyflakeGenerator

当机器号空间优先级高于单机瞬时吞吐时，可以使用
`SonyflakeGenerator`。

```rust
use qubit_id::{IdGenerator, SonyflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SonyflakeGenerator::new(65_535)?;

    let id = generator.next_id()?;

    assert_eq!(generator.extract_machine_id(id), 65_535);
    println!("{id}");

    Ok(())
}
```

对于自定义部署，也可以显式配置序列位、机器位、时间单位和起始时间。

```rust
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{IdGenerator, SonyflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SonyflakeGenerator::builder(15)
        .bits_sequence(10)
        .bits_machine(14)
        .time_unit(Duration::from_millis(1))
        .start_time(UNIX_EPOCH + Duration::from_secs(1_735_689_600))
        .build()?;

    let id = generator.next_id()?;

    assert_eq!(generator.bits_sequence(), 10);
    assert_eq!(generator.bits_machine(), 14);
    assert_eq!(generator.extract_machine_id(id), 15);

    Ok(())
}
```

### MicaUuidLikeGenerator 和便捷函数

需要随机 128 位值和 UUID-like 小写文本格式时，使用
`MicaUuidLikeGenerator`。如果只需要字符串，可以直接使用便捷函数。

```rust
use qubit_id::{
    IdGenerator, MicaUuidLikeGenerator, fast_simple_uuid_like, fast_uuid_like,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = MicaUuidLikeGenerator::new();

    let value = generator.next_id()?;
    let canonical = generator.format_id(&value);
    let compact = MicaUuidLikeGenerator::format_simple_uuid_like(value);

    let random_canonical = fast_uuid_like()?;
    let random_compact = fast_simple_uuid_like()?;

    println!("{canonical} {compact} {random_canonical} {random_compact}");
    Ok(())
}
```

## 算法说明

`QubitSnowflakeGenerator` 是 Qubit Rust 服务默认的 Snowflake 风格生成器。
它使用固定高位头部：

```text
[mode:1][precision:1][timestamp][host:9][sequence]
```

各字段位宽如下：

| 字段 | 位宽 | 说明 |
| --- | --- | --- |
| `mode` | 1 位 | 编码 ID 排序模式：顺序模式或打散模式。 |
| `precision` | 1 位 | 编码时间精度：毫秒精度或秒精度。 |
| `timestamp` | 毫秒精度 41 位；秒精度 31 位 | 从配置 epoch 开始经过的时间片数量。 |
| `host` | 9 位 | 主机编号，取值范围 `0..=511`。 |
| `sequence` | 毫秒精度 12 位；秒精度 22 位 | 同一时间片内的递增序列号。 |

固定 `mode` 和 `precision` 的位置后，不需要先知道 timestamp 和 sequence 的位宽，也能读取这两个头部字段。

这个布局优先保证头部自描述，便于在解析时直接识别 ID 的 mode 和 precision。

Qubit 打散模式的 ID 可能设置第 63 位，因此数值可能超过 `i64::MAX`。应使用
无符号 64 位整数、十进制字符串或二进制数据存储；跨越 JavaScript 风格的
安全整数边界时，应使用字符串。

该 64 位布局既不预留符号位，也不预留版本字段。这是为了保留容量和吞吐量而
做出的有意取舍。未来若引入不兼容布局，必须使用新的显式类型或 API，不能
静默改变当前布局。

对任意 `u64` 解码只会按照布局提取字段。它不能证明该数值由本生成器产生，
也不是鉴权或格式校验操作。

### 三种 Snowflake 生成器如何选择

| 生成器 | 优势 | 取舍 |
| --- | --- | --- |
| `QubitSnowflakeGenerator` | 固定高位包含 `mode` 和 `precision`，解析时不需要先知道完整布局；支持毫秒/秒两种精度，默认秒精度可在单主机上提供更大的序列空间；支持顺序模式和打散模式；对小幅时钟回拨有默认容忍。 | 使用 Qubit 自有布局；host 为 9 位，最多 512 个主机编号。 |
| `SnowflakeGenerator` | 经典 41 位毫秒时间、10 位节点、12 位序列布局，结构直观，适合需要传统 Snowflake 形态的场景。 | 布局固定，不编码 mode/precision；遇到时钟回拨会直接返回错误；没有打散模式。 |
| `SonyflakeGenerator` | 默认 63 位 ID、10 ms 时间单位、16 位机器号，适合机器号空间更大的部署；序列位和机器位可配置。 | 默认每个时间片只有 8 位序列，单机瞬时吞吐低于毫秒级 Snowflake；10 ms 时间单位下时间顺序粒度更粗。 |

通常优先选择 `QubitSnowflakeGenerator`：它仍然生成紧凑的 `u64` 数字 ID，但把布局元信息编码到固定高位，后续解析、排查和演进更直接。需要传统 41/10/12 布局时再选择 `SnowflakeGenerator`；机器号空间明显优先于单机瞬时吞吐时，可以选择 `SonyflakeGenerator`。

### MicaUuidLikeGenerator

`MicaUuidLikeGenerator` 本质上只是一个随机数生成器，只是模仿标准 UUID 的文本形态。它使用 128 位随机数，并格式化为小写 UUID-like 文本。它不会重写 RFC UUID 版本位或 variant 位，因此不应当被当作标准 UUID v4 生成器使用。

UUID-like 格式化逻辑参考 Mica 的快速 UUID 辅助函数，以及
[`StringUtil`](https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L335)
中的
[`formatUnsignedLong`](https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L348)
格式化辅助函数。
Mica 的 UUID 压测说明见
[mica-jmh wiki](https://github.com/lets-mica/mica-jmh/wiki/uuid)。

## 项目边界

- 本 crate 只负责本地 ID 生成，不负责分布式节点发现。
- Qubit Snowflake 会在配置的回拨容忍范围内等待；经典 Snowflake 和
  Sonyflake 对任何已观测到的回拨都会返回错误。
- `QubitSnowflakeGenerator` 使用自己的固定头部 Snowflake 布局。
- `SnowflakeGenerator` 和 `SonyflakeGenerator` 适合服务主动选择这些布局时使用。

## 贡献

欢迎提交 issue 和 pull request。

请保持变更聚焦，便于 review：

- bug、设计问题或较大的功能提议请先开 issue
- pull request 尽量只包含一个行为变更、修复或文档更新
- 提交前运行 `./ci-check.sh`
- 修改运行时行为时补充测试
- 修改公共 API 行为时更新 README

提交贡献即表示你同意该贡献使用本项目相同的许可证发布。

## 许可证

本项目使用 [Apache License, Version 2.0](LICENSE) 许可证。
