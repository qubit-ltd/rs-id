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

它包含一个统一的 `IdGenerator<T>` trait，并提供数据库友好的 Snowflake
ID、Sonyflake 风格 ID，以及和 Java 快速 UUID 辅助函数行为一致的随机 UUID
字符串。

## 适用场景

当你需要以下能力时，可以使用 `qubit-id`：

- 兼容 Java `common-id` 的 Snowflake ID，用于数据库记录主键
- 经典 Snowflake 布局的 64 位数字 ID
- Sonyflake 风格 ID，在较小序列位宽下换取更长可用时间和更大机器号空间
- 与现有 Java 辅助函数行为一致的快速 UUID 字符串
- 通过同一个 trait 同时获得强类型 ID 和字符串表示

## 安装

```toml
[dependencies]
qubit-id = "0.1.0"
```

## 快速开始

```rust
use qubit_id::{IdGenerator, QubitSnowflakeGenerator, UuidGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    let uuid = UuidGenerator::new();
    let uuid_value: u128 = uuid.next_id()?;
    let uuid_text = uuid.next_string()?;

    println!("{id} {id_text} {uuid_value} {uuid_text}");
    Ok(())
}
```

## 核心 API

| 类型 | 作用 |
| --- | --- |
| `IdGenerator<T>` | 统一的强类型 ID 生成和字符串格式化 trait。 |
| `QubitSnowflakeGenerator` | 兼容 Java `common-id` 的 Snowflake 生成器。 |
| `QubitSnowflakeBuilder` | 构造和解析 Java 兼容 Snowflake 位布局。 |
| `SnowflakeGenerator` | 经典 41 位时间、10 位节点、12 位序列 Snowflake 生成器。 |
| `SonyflakeGenerator` | 支持配置序列位和机器位的 Sonyflake 风格生成器。 |
| `UuidGenerator` | 快速随机 128 位 UUID 形态生成器。 |
| `fast_uuid` | 生成小写标准 UUID 字符串。 |
| `fast_simple_uuid` | 生成小写 32 位十六进制 UUID 字符串。 |

## 算法说明

`QubitSnowflakeGenerator` 是 Qubit 服务的默认选择。它保留 Java
`common-id` 的位布局，包括模式、精度、9 位 host、timestamp 和 sequence。

`SnowflakeGenerator` 适合明确需要经典 Snowflake 布局的服务：41 位毫秒时间、
10 位节点和 12 位序列。

`SonyflakeGenerator` 采用 Sonyflake 风格取舍：默认使用 63 位 ID、10 ms 时间
单位、更大的机器号字段。它牺牲单机瞬时吞吐，换取更长可用时间和更大的机器号
空间。

`UuidGenerator` 刻意匹配 Java 快速 UUID 辅助函数：使用 128 位随机数，并格式化
为小写 UUID 文本。它不会重写 RFC 版本位或 variant 位。

## 项目边界

- 本 crate 只负责本地 ID 生成，不负责分布式节点发现。
- 时钟回拨会在配置容忍范围内等待，超过范围后返回明确错误。
- 需要兼容 Java 现有 ID 时，应优先使用 `QubitSnowflakeGenerator`。
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
