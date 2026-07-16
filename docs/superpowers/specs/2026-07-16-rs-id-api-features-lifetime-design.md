# rs-id API、Feature 与寿命治理设计

## 背景

`rs-id` 目前同时默认编译 Qubit Snowflake、Classic Snowflake、Sonyflake 和
Mica UUID-like 四类能力。Qubit 已经把无状态位布局提取为
`QubitSnowflakeLayout` 和 `QubitSnowflakeParts`，而 Classic 与 Sonyflake
仍把 compose、extract 和布局查询放在有状态生成器上。仅解析已有 ID 的代码
因此也需要构造包含时钟、锁和分配状态的生成器。

当前所有算法还共享一个无 feature 边界的默认构建；随机 ID 使用者会同时引入
Snowflake 所需的时钟与互斥锁依赖。生成器虽然能够在第一次生成时报告
`TimestampOverflow`，却不能直接查询到期时间，也不会在构造时阻止一个已经
过期的配置启动应用。

本轮允许公开 API 破坏性调整，不保留旧 compose/extract 路径。目标是在包正式
发布前收敛边界，而不是为未出现的下游使用保留兼容层。

## 目标

- 默认只启用 Qubit Snowflake；Classic Snowflake、Sonyflake 和 UUID-like
  分别通过 feature 开启。
- 为 Classic 与 Sonyflake 建立独立的 Layout 和 Parts 类型，使无状态位运算
  不依赖生成器。
- 为所有 Snowflake 系列提供明确、可查询的到期时间。
- 构造生成器时读取配置的 `WallClock`；若当前时间已经到达或超过到期边界，
  立即 panic。
- 统一审计 crate 内所有 Rust 类型、函数和方法的 `#[must_use]` 与 inline
  属性。
- 增加 Mica UUID-like 与标准 UUID v4 的可复现 benchmark，不设置不稳定的
  性能阈值。
- 保持既有 ID 位编码、回拨、重启策略、阻塞和错误传播语义不变，除本设计明确
  调整的构造期过期行为与公开 API 路径外。

## 非目标

- 不提供持久化序列水位、节点租约或跨进程协调。
- 不增加异步运行时绑定；调用方继续使用非休眠的 `try_next_id()`。
- 不把进程内 counter、业务 ID 或临时文件名策略并入 `rs-id`。
- 不修改 Mica UUID-like 的随机位语义，也不把它伪装成 RFC UUID v4。
- 不进行无锁化、分段序列分配或其他未由 benchmark 驱动的性能重写。

## 方案比较与选择

### 布局 API

1. **独立 Layout + Parts 类型（采用）**：Classic 与 Sonyflake 按 Qubit 的边界
   拆分；生成器只负责时间观察和唯一分配。公开类型较多，但职责最清晰。
2. **只在生成器增加 `decode` 包装**：文件变化较少，但解析仍要求构造运行时
   对象，无法解决状态与位布局耦合。
3. **引入泛型 Layout trait**：能统一方法名，但三种算法的 header、自描述能力
   和时间单位不同，抽象会把差异转移到复杂关联类型中，当前没有下游需求支撑。

选择方案 1。包尚未正式发布，因此直接删除 Classic/Sonyflake 生成器上的旧
compose/extract API，不增加弃用层。

### Feature 结构

1. **默认 Qubit、其他算法独立开关（采用）**：依赖和 API 最符合使用意图，
   feature 组合也能由现有 CI feature-check 覆盖。
2. **Qubit 永远编译、其他算法开关**：`--no-default-features` 仍无法得到纯 trait
   核心，依赖边界不完整。
3. **把 Classic 与 Sonyflake 合并成 compatibility feature**：矩阵更小，但无法
   单独选择算法，与本轮明确要求不符。

选择方案 1。feature 定义为：

```toml
[features]
default = ["qubit-snowflake"]
qubit-snowflake = ["dep:parking_lot", "dep:qubit-clock"]
classic-snowflake = ["dep:parking_lot", "dep:qubit-clock"]
sonyflake = ["dep:parking_lot", "dep:qubit-clock"]
uuid = ["dep:getrandom"]
```

`thiserror` 保持基础依赖；`getrandom`、`parking_lot` 和 `qubit-clock` 改为
optional。`--no-default-features` 只提供 `IdGenerator`、`GenerationOutcome`
和通用错误模型；默认构建提供这些核心 API 与 Qubit Snowflake。

### 到期时间表示

1. **构造时验证并缓存具体 `SystemTime`（采用）**：生成器的 `expires_at()`
   无错误、无重算，运维调用最直接。
2. **返回 `Option<SystemTime>`**：能表示超出平台时间范围，但把无效配置推给
   每个调用者处理。
3. **只返回有效 `Duration`**：计算简单，却要求调用方重复与 epoch/start time
   相加，也不便直接配置监控告警。

选择方案 1。布局根据时间原点计算排他的到期边界；如果该边界无法由
`SystemTime` 表示，builder 返回新的配置错误。只有“配置可以表示，但当前时间
已经到期”属于不可恢复的启动状态，并按要求 panic。

## Feature 与模块边界

`lib.rs` 按 feature 条件声明和重导出模块：

- `qubit-snowflake`：Qubit layout、parts、generator、builder、mode 和
  precision。
- `classic-snowflake`：Classic layout、parts、generator 和 builder，并复用
  Snowflake 内部状态机与 restart policy。
- `sonyflake`：Sonyflake layout、parts、generator 和 builder，并复用
  Snowflake 内部状态机与 restart policy。
- `uuid`：`MicaUuidLikeGenerator`、`fast_uuid_like` 和
  `fast_simple_uuid_like`。

`snowflake` 模块在任意 Snowflake feature 开启时存在。`RestartPolicy` 和共享
内部状态机也在任意 Snowflake feature 下编译；只属于某一算法的类型在其
feature 下声明和重导出。集成测试模块使用相同 cfg，确保默认测试不会引用
关闭的 API。

## Layout 与 Parts API

### Classic Snowflake

新增 `SnowflakeLayout`，持有 Classic 固定 41/10/12 布局所需的 `node_id`，
提供：

```rust
pub fn new(node_id: u64) -> Result<Self, IdError>;
pub const fn node_id(&self) -> u64;
pub const fn max_timestamp(&self) -> u64;
pub const fn max_sequence(&self) -> u64;
pub fn compose(&self, timestamp: u64, sequence: u64)
    -> Result<u64, IdError>;
pub const fn decode(id: u64) -> SnowflakeParts;
pub fn expires_at(&self, epoch: SystemTime) -> Result<SystemTime, IdError>;
```

新增 `SnowflakeParts`，保存并提供 `timestamp`、`node_id`、`sequence` getter。
`SnowflakeGenerator` 改为持有 `SnowflakeLayout`，公开 `layout()`、`epoch()` 和
`expires_at()`；删除生成器上的 compose、extract、node/max 查询，使布局知识
只有一个所有者。

### Sonyflake

新增 `SonyflakeLayout`，持有 `machine_id`、sequence/machine 位宽、推导出的
time 位宽和 `time_unit`。它负责现有位宽、机器 ID 和时间单位校验，提供布局
getter、max 查询、`compose(elapsed_time, sequence)`、`decode(id)` 以及
`expires_at(start_time)`。

新增 `SonyflakeParts`，保存并提供 `elapsed_time`、`sequence`、`machine_id`
getter。`SonyflakeGenerator` 改为持有 `SonyflakeLayout`、`start_time` 和缓存的
到期时间；删除生成器上的 compose、extract 和布局 getter。

### Qubit

保留 `QubitSnowflakeLayout` 与 `QubitSnowflakeParts` 的现有编码 API，并增加：

```rust
pub fn expires_at(&self, epoch: SystemTime) -> Result<SystemTime, IdError>;
```

`QubitSnowflakeGenerator` 增加无错误 getter：

```rust
pub const fn expires_at(&self) -> SystemTime;
```

三种生成器的 `expires_at()` 都返回排他边界：当 `now >= expires_at` 时已过期；
边界前一个可表示时刻仍有效。

## 构造期验证与错误处理

共享内部辅助函数根据 `time_unit * (max_timestamp + 1)` 计算有效期，使用
`u128` 纳秒做 checked arithmetic，再转换为 `Duration` 和 `SystemTime`。
无法表示时返回新的 `IdError::ExpirationTimeOverflow`，字段携带时间原点、
时间单位和最大时间戳，便于定位错误配置。

每个 builder 的构造顺序为：

1. 校验 layout 配置；
2. 计算并缓存 `expires_at`；
3. 从注入的 `WallClock` 读取一次当前时间；
4. Sonyflake 保留 `start_time > now` 时返回 `StartTimeAhead` 的既有语义；
5. 当 `now >= expires_at` 时 panic，消息包含算法、当前时间和到期时间；
6. 否则构造生成器及空分配状态。

构造时当前时间早于 epoch/start time 不 panic，仍由第一次生成返回既有的
`TimeBeforeEpoch`。生成器存活期间越过边界时，现有生成路径继续返回
`TimestampOverflow`。

`new(...)` 委托 builder，因此也具有相同 panic 行为。所有相关 rustdoc 增加
精确的 `# Panics`、`# Errors` 与排他边界说明。

## `must_use` 与 inline 审计

按语义而不是按返回类型机械添加属性：

- Layout、Parts、generator、builder、配置枚举和 generation outcome 等被直接
  丢弃通常表示错误的领域值，优先使用类型级 `#[must_use]`。
- 返回 primitive、`String`、引用或决策布尔值的 getter、格式化、解析和内部
  状态判断使用函数级 `#[must_use]`。
- 已由 `Result`、`Option`、`Future` 或 must-use 返回类型保护的方法不重复标记。
- 为对外可观察的 must-use 契约增加 `compile_fail` doctest，并通过先失败、后
  通过的 TDD 顺序验证警告确实来自目标属性。
- getter、setter、纯委托和极薄包装使用 `#[inline(always)]`；其他短小且分支少
  的函数使用 `#[inline]`；循环、长函数和分支密集状态机不使用 inline 属性。
- 审计覆盖公开 API、trait 方法、restricted API、私有辅助函数和新增代码。

## UUID 对比 Benchmark

增加自定义 harness benchmark，与现有 benchmark 风格保持一致，不引入
Criterion。`uuid` 作为仅 benchmark/test 使用的 dev-dependency，开启标准
UUID v4 生成能力。新 benchmark 设置 `required-features = ["uuid"]`，比较：

- 仅生成 128 位值：Mica `next_id()` 与 `Uuid::new_v4()`；
- 生成并格式化 canonical 文本：Mica `next_string()` 与标准 UUID hyphenated
  文本；
- 生成并格式化 compact 文本：`fast_simple_uuid_like()` 与标准 UUID simple
  文本。

每个 case 使用相同预热次数、样本次数和每样本迭代数，输出 min/median/max
吞吐量。benchmark 不包含通过/失败阈值，因为调度、随机源和硬件会影响结果。
标准 UUID v4 会设置 version/variant bits，因此这是面向实际 API 成本的比较，
不是完全相同随机位语义的微基准。

## 测试与验证

实施严格按测试先行：

1. 先为 feature cfg、Classic/Sonyflake Layout/Parts、到期查询和构造期 panic
   写失败测试，并确认失败原因是目标 API 或行为尚不存在；
2. 写最小实现使单项测试通过；
3. 再进行 generator 委托、删除旧 API、must-use/inline 和文档整理；
4. UUID benchmark 先完成编译级测试，再运行并检查输出，不做性能断言。

关键边界测试包括：

- compose/decode 往返、字段最大值和越界错误；
- `expires_at` 是排他边界；边界前可构造，边界上和边界后 panic；
- 无法表示到期时间时返回配置错误；
- 默认 feature 只暴露 Qubit，单 feature 和 all-features 构建均成立；
- 关闭默认 feature 时核心 trait/error API 仍可编译；
- 现有回拨、序列耗尽、重启策略、sleep failure 和并发唯一性测试继续成立。

最终验证严格按仓库顺序运行：`./align-ci.sh`，然后 `./ci-check.sh`；只有 CI
报告覆盖率低于阈值时才运行 `./coverage.sh json`。此外编译并运行新的 UUID
benchmark，记录实际命令与结果。

## 破坏性变更清单

- 默认构建不再包含 Classic Snowflake、Sonyflake 和 UUID-like API。
- Classic/Sonyflake 的 compose、extract 与布局 getter 从 generator 移至新
  Layout/Parts 类型。
- generator 的内部字段改为 layout + 缓存到期时间。
- 已过期配置从“构造成功、首次生成返回错误”变为“构造时 panic”。
- 新增的 must-use 属性可能使启用 `deny(unused_must_use)` 的调用方出现编译错误。

这些变更均在包尚未正式发布、当前 `rs-*` 工作区没有直接消费者的前提下接受。
