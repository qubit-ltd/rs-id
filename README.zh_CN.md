# Qubit ID (`rs-id`)

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-id.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-id)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-id/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-id?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Docs.rs](https://docs.rs/qubit-id/badge.svg)](https://docs.rs/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

文档：[API Reference](https://docs.rs/qubit-id)

`qubit-id` 为 Rust 服务提供 ID 生成工具。

它包含一个统一的 `IdGenerator<T>` trait，并提供数据库友好的 Snowflake ID、Sonyflake 风格 ID，以及快速随机 UUID-like 字符串。

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
qubit-id = "0.1.0"
```

## 快速开始

```rust
use qubit_id::{FastUuidLikeGenerator, IdGenerator, QubitSnowflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    let uuid_like = FastUuidLikeGenerator::new();
    let uuid_like_value: u128 = uuid_like.next_id()?;
    let uuid_like_text = uuid_like.next_string()?;

    println!("{id} {id_text} {uuid_like_value} {uuid_like_text}");
    Ok(())
}
```

## 核心 API

| 类型 | 作用 |
| --- | --- |
| `IdGenerator<T>` | 统一的强类型 ID 生成和字符串格式化 trait。 |
| `QubitSnowflakeGenerator` | Qubit 固定头部 Snowflake 生成器。 |
| `QubitSnowflakeBuilder` | 构造和解析 Qubit Snowflake 位布局。 |
| `SnowflakeGenerator` | 经典 41 位时间、10 位节点、12 位序列 Snowflake 生成器。 |
| `SonyflakeGenerator` | 支持配置序列位和机器位的 Sonyflake 风格生成器。 |
| `FastUuidLikeGenerator` | 快速随机 128 位 UUID-like 生成器。 |
| `fast_uuid_like` | 生成小写标准形态 UUID-like 字符串。 |
| `fast_simple_uuid_like` | 生成小写 32 位十六进制 UUID-like 字符串。 |

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

固定 `mode` 和 `precision` 的位置后，不需要先知道 timestamp 和 sequence 的
位宽，也能读取这两个头部字段。

这个布局优先保证头部自描述，便于在解析时直接识别 ID 的 mode 和 precision。

### 三种 Snowflake 生成器如何选择

| 生成器 | 优势 | 取舍 |
| --- | --- | --- |
| `QubitSnowflakeGenerator` | 固定高位包含 `mode` 和 `precision`，解析时不需要先知道完整布局；支持毫秒/秒两种精度，默认秒精度可在单主机上提供更大的序列空间；支持顺序模式和打散模式；对小幅时钟回拨有默认容忍。 | 使用 Qubit 自有布局；host 为 9 位，最多 512 个主机编号。 |
| `SnowflakeGenerator` | 经典 41 位毫秒时间、10 位节点、12 位序列布局，结构直观，适合需要传统 Snowflake 形态的场景。 | 布局固定，不编码 mode/precision；遇到时钟回拨会直接返回错误；没有打散模式。 |
| `SonyflakeGenerator` | 默认 63 位 ID、10 ms 时间单位、16 位机器号，适合机器号空间更大的部署；序列位和机器位可配置。 | 默认每个时间片只有 8 位序列，单机瞬时吞吐低于毫秒级 Snowflake；10 ms 时间单位下时间顺序粒度更粗。 |

通常优先选择 `QubitSnowflakeGenerator`：它仍然生成紧凑的 `u64` 数字 ID，但把布局元信息编码到固定高位，后续解析、排查和演进更直接。需要传统 41/10/12 布局时再选择 `SnowflakeGenerator`；机器号空间明显优先于单机瞬时吞吐时，可以选择 `SonyflakeGenerator`。

`FastUuidLikeGenerator` 使用 128 位随机数，并格式化为小写 UUID-like 文本。它不会重写 RFC UUID 版本位或 variant 位，因此不应当被当作标准 UUID v4 生成器使用。

## 项目边界

- 本 crate 只负责本地 ID 生成，不负责分布式节点发现。
- 时钟回拨会在配置容忍范围内等待，超过范围后返回明确错误。
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
