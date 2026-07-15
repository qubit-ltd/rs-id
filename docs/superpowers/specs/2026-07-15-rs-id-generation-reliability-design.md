# rs-id 生成可靠性与公共 API 重构设计

## 背景

`rs-id` 当前提供 Qubit Snowflake、Classic Snowflake、Sonyflake 和
Mica UUID-like 生成器。现有 Snowflake 系列实现能够保证同一个存活实例内
的唯一性，但实例状态不持久化：使用同一节点身份的进程在同一逻辑时间片
内重启时，序列会重新从零开始，因而可能重复。当前阻塞生成接口还直接使用
真实时钟与线程休眠，使序列耗尽、启动等待和时钟回拨测试依赖真实时间。

此外，Qubit Snowflake 在将原始墙上时间量化为秒或毫秒后才判断回拨。
秒精度模式下，同一秒内的物理时钟回拨会被量化掩盖，且现有
`max_skew_millis` 名称与实际判断精度不一致。`IdGenerator<T>` 的默认
格式化方法也要求 `T: Display`，与 trait 允许实现自定义格式的文档契约冲突。

本设计统一解决上述可靠性、可测试性和 API 契约问题，同时明确保留本轮不做
的持久化、位布局和无锁性能优化。

## 目标

- 为所有 Snowflake 系列生成器增加显式 `RestartPolicy`，默认使用
  `Immediate`，并准确说明可能重复的条件。
- 增加绝不休眠的 `try_next_id()`，让调用方可以自行决定重试、调度或放弃。
- 使用 `qubit-clock` 注入墙上时钟与阻塞 sleeper，使所有等待路径都可确定性
  测试。
- 在时间量化前检测原始时钟回拨，修复秒精度模式下被掩盖的回拨。
- 将 `IdGenerator` 改为关联 ID 类型，并允许不实现 `Display` 的 ID 提供专用
  格式。
- 改善错误上下文和错误链，整理已确认的 Rust 代码与测试组织问题。
- 在中英文 README 与 rustdoc 中同步记录重启、存储和布局限制。

## 非目标

- 不提供持久化生成状态，也不定义 `PersistentState`。
- 不预留符号位、布局版本位或其他新位段；现有 ID 数值布局保持不变。
- 不解决相同节点身份的多个实例并行生成导致的冲突。
- 不在本轮将 mutex 状态机改为原子或分段分配算法。
- 不增加异步生成接口，也不删除同步阻塞接口。
- 不改变 Qubit、Classic Snowflake 或 Sonyflake 的编码、解码结果。

## 方案选择

采用“统一非阻塞状态机 + 阻塞适配层”方案：状态机只产生 ID、可恢复等待结果
或不可恢复错误；`next_id()` 是在该状态机之上的同步等待适配。相比只给现有
代码打补丁，这能让三种 Snowflake 的重启、序列耗尽和时钟回拨语义一致，并
消除测试中的真实休眠。相比删除阻塞接口，它保留了多数同步调用方需要的简单
用法。

## 公共 API

### RestartPolicy

新增公共枚举：

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestartPolicy {
    #[default]
    Immediate,
    WaitNextSlice,
}
```

三种 Snowflake 生成器的 builder 都提供
`restart_policy(RestartPolicy)`。`new(...)` 仍是使用默认配置的便捷入口，
并委托给 builder。

`Immediate` 在第一次成功生成时直接使用当前逻辑时间片和序列零。默认采用
该策略，以保持立即可用和现有吞吐特征。以下条件同时成立时可能产生重复：

1. 两个实例使用相同 ID 布局、时间原点和有效节点身份；
2. 两个实例在同一逻辑时间片内成功生成；
3. 两个实例分配的序列范围发生重叠。

同一身份快速重启会因序列状态丢失而满足这些条件；相同身份的并行实例也可能
满足这些条件。

`WaitNextSlice` 的第一次 `try_next_id()` 调用记录当前编码时间片作为启动
基线，不分配 ID，并返回距离下一时间片边界的等待时长。后续调用只有在编码
时间片严格大于基线时才激活分配，进入新时间片后从序列零开始。

该策略可以防止“旧实例已经停止、新实例随后启动”的无状态同片重启重复，
但不能防止旧实例仍在运行或再次启动时的相同身份并行冲突。它也可能在无法
读取旧实例状态时保守地多等待一个完整时间片。

### GenerationOutcome

新增公共枚举：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum GenerationOutcome<T> {
    Generated(T),
    RetryAfter(Duration),
}
```

`GenerationOutcome::map` 转换 `Generated` 中的值，并原样保留
`RetryAfter`。返回的等待时长必须大于零；状态机不得用零时长制造忙循环。

### IdGenerator

将泛型 trait 改为关联 ID 类型：

```rust
pub trait IdGenerator {
    type Id;
    type Error: std::error::Error + Send + Sync + 'static;

    fn try_next_id(
        &self,
    ) -> Result<GenerationOutcome<Self::Id>, Self::Error>;

    fn next_id(&self) -> Result<Self::Id, Self::Error>;

    fn format_id(&self, id: &Self::Id) -> String;

    fn next_string(&self) -> Result<String, Self::Error> {
        self.next_id().map(|id| self.format_id(&id))
    }

    fn try_next_string(
        &self,
    ) -> Result<GenerationOutcome<String>, Self::Error> {
        self.try_next_id()
            .map(|outcome| outcome.map(|id| self.format_id(&id)))
    }
}
```

`format_id` 成为必须实现的方法，不再要求 `Self::Id: Display`。内置数值
生成器返回十进制字符串，Mica UUID-like 生成器继续返回其 UUID-like 格式。

`try_next_id()` 绝不调用 sleeper。它只返回：

- `Generated(id)`：已经完成唯一 ID 分配；
- `RetryAfter(duration)`：当前状态可恢复，但调用方需要等待后重试；
- `Err(error)`：当前调用无法通过等待恢复。

`next_id()` 对 Snowflake 系列反复调用同一非阻塞状态机；收到
`RetryAfter` 后使用注入的 `BlockingSleeper` 等待。Mica 不需要等待，两个
生成入口都直接生成。

这是明确允许的源代码级破坏性变更，但不会改变任何现有 ID 的位布局。

## Builder 与时钟依赖

新增 `SnowflakeGeneratorBuilder`，并使三种 Snowflake builder 都支持以下
公共配置：

```rust
SnowflakeGenerator::builder(node_id: u64) -> SnowflakeGeneratorBuilder;
QubitSnowflakeGenerator::builder(host: u64)
    -> QubitSnowflakeGeneratorBuilder;
SonyflakeGenerator::builder(machine_id: u64)
    -> SonyflakeGeneratorBuilder;

fn restart_policy(self, policy: RestartPolicy) -> Self;
fn wall_clock(self, clock: Arc<dyn WallClock>) -> Self;
fn blocking_sleeper(self, sleeper: Arc<dyn BlockingSleeper>) -> Self;
```

每种生成器只保留 `new(identity)` 和 `builder(identity)` 两条构造路径：`new`
委托默认 builder，builder 负责所有自定义配置。现有多参数
`with_epoch(...)`、`with_options(...)` 和 `with_clock(...)` 构造入口删除，
避免维护并行配置 API；compose、decode、extract 和只读配置访问器不受影响。

Qubit builder 的容忍回拨配置改为：

```rust
fn max_clock_skew(self, max_clock_skew: Duration) -> Self;
```

对应 getter 返回 `Duration`。`DEFAULT_MAX_SKEW_MILLIS` 替换为语义准确的
`DEFAULT_MAX_CLOCK_SKEW: Duration`，默认值仍为三秒。Classic Snowflake
和 Sonyflake 保持零容忍，不额外暴露容忍回拨配置。

现有接收闭包的 `clock(...)`/`with_clock(...)` 入口由上述 rs-clock trait
object 配置取代。默认配置使用 `StdWallClock`，以及由
`StdMonotonicClock` 驱动的 `StdBlockingSleeper`。Cargo 直接依赖：

```toml
qubit-clock = { version = "0.9", path = "../rs-clock" }
thiserror = "2.0"
```

测试使用同一个 `ManualMonotonicClock` 驱动 `ManualWallClock` 与
`ManualBlockingSleeper`，确保一次阻塞等待会推进测试墙上时间而不发生真实
休眠。墙上时钟和 sleeper 分开注入，是为了允许生产环境使用系统墙上时间
观察时间戳，同时使用单调时钟实施等待。

## 内部状态机

三种 Snowflake 共享同一个私有分配状态模型。状态至少保存：

- 已观察到的原始 elapsed time 高水位；
- 当前已分配的逻辑 `TimeSlice` 及序列；
- `WaitNextSlice` 的启动基线和是否已经激活。

一次 `try_next_id()` 的数据流为：

1. 获取生成器 mutex；
2. 在 mutex 内读取 `WallClock`，避免并发调用先读后锁导致虚假回拨；
3. 将 `SystemTime` 转为相对配置 epoch/start time 的原始 `Duration`；
4. 将原始 `Duration` 与观测高水位比较并处理回拨；
5. 将原始时间量化为布局使用的逻辑时间片；
6. 执行启动门槛、时间片切换或序列递增；
7. 在分配成功时组合 ID，或者返回精确的 `RetryAfter`；
8. 释放 mutex 后将结果返回，绝不在持锁期间休眠。

高水位记录所有正常向前的时钟观测，而不仅是成功生成 ID 的时刻。这样在序列
耗尽等待期间发生的物理回拨也不会被遗漏。检测到回拨时不降低高水位。

### 回拨处理

回拨判断发生在秒、毫秒或自定义 Sonyflake 单位量化之前：

- Qubit 回拨不超过 `max_clock_skew` 时返回等于原始回拨量的
  `RetryAfter`；超过容忍值时返回 `ClockMovedBackwards`。
- Classic Snowflake 和 Sonyflake 的容忍值为零，任何已观察到的原始回拨
  都返回 `ClockMovedBackwards`，即使量化后的时间片没有变化。
- 允许的回拨不会修改时间片、序列或原始高水位；只有时钟追平后才能继续。

### 其他等待原因

- 序列耗尽时，`RetryAfter` 是从当前原始时间到下一逻辑时间片边界的时长。
- `WaitNextSlice` 尚未跨过启动基线时，返回到基线下一边界的剩余时长。
- 如果时间已经超过布局上限、早于 epoch/start time，或 builder 配置无效，
  直接返回错误。

## 错误模型

`IdError` 使用 `thiserror` 派生 `Error` 并标记 `#[non_exhaustive]`。不再为了
保持 `Clone`/`Eq` 而丢失底层错误；测试改为匹配有意义的 variant 和字段。

关键调整包括：

- `ClockMovedBackwards` 携带相对 epoch/start time 的
  `last_elapsed: Duration`、`current_elapsed: Duration`、`skew: Duration`
  和 `max_skew: Duration`；
- `TimeBeforeEpoch` 携带实际时间和配置 epoch；
- `StartTimeAhead` 携带配置 start time 和 builder 观察到的当前时间；
- `RandomSourceUnavailable` 通过 `#[source]` 保留 `getrandom::Error`；
- 新增 `SleepFailed`，通过 `#[source]` 保留 `qubit_clock::TimeError`。

只有阻塞 `next_id()` 可能返回 `SleepFailed`；`try_next_id()` 不调用 sleeper，
因此不会产生该错误。

## 文件与模块边界

公共类型各自放在独立文件中：

- `src/generation_outcome.rs`
- `src/snowflake/restart_policy.rs`
- `src/snowflake/snowflake_generator_builder.rs`

Snowflake 私有状态类型移动到 `src/snowflake/internal/`，由私有
`internal/mod.rs` 组织。现有 `time_slice.rs` 和
`time_slice_reservation.rs` 移入该目录，并按最终职责拆分时钟观测、生成状态
和分配决策；每个 Rust 文件只定义一个主要类型。

冗余的集成测试子目录改为与源码文件一一对应的平铺测试文件：

- `tests/snowflake/qubit_snowflake_generator_tests.rs`
- `tests/snowflake/snowflake_generator_tests.rs`
- `tests/snowflake/sonyflake_generator_tests.rs`
- `tests/uuid/mica_uuid_like_generator_tests.rs`

旧目录中的同名测试内容迁移后删除冗余 `mod.rs`。被 rs-clock 替代的自制测试
时钟删除；仍需复用的测试辅助类型按一个类型一个文件放在对应测试 support
模块。benchmark 中的辅助类型也遵守相同组织约束，但不改变测量算法、参数或
已记录的性能结论。

本轮同时修正已确认的 Rust 风格问题：完整记录私有字段与函数、rustdoc 使用
统一的 `# Arguments`/`# Returns`/`# Errors` 标题、按可见性排列 inherent
impl 方法，并按项目规则放置 `#[inline]` 等属性。这些整理不得引入与本设计
无关的行为变化。

## 文档约束

`README.md`、`README.zh_CN.md` 和相关 rustdoc 必须同步说明：

- `Immediate` 是默认值及其精确重复条件；
- `WaitNextSlice` 仅保护顺序重启，不保护相同身份并行实例；
- `try_next_id()` 不阻塞，`next_id()` 在可恢复状态下可能无限等待；
- 停滞的墙上时钟或不推进墙上时间的自定义 sleeper 可能使阻塞调用一直重试；
- Qubit Spread 模式可能设置 bit 63，不能无损存入有符号 64 位字段；应使用
  无符号 64 位、字符串或二进制表示，JavaScript 等安全整数范围较小的环境
  应使用字符串传输；
- 当前布局没有符号保留位或版本位，这是为保留单机吞吐与 ID 空间作出的明确
  取舍；未来不兼容布局必须使用新的显式类型/API，而不能静默改变现有解码；
- 对任意 `u64` 的结构化 decode 只提取位字段，不证明该值由对应生成器产生，
  也不是格式真实性验证。

英文和中文文档的行为、默认值、示例和限制必须等价。

## 测试策略

实现遵循测试先行。时钟回拨修复的第一项代码改动必须是失败回归测试：在 Qubit
秒精度模式中先成功生成，随后让原始墙上时间在同一编码秒内回拨，验证
`try_next_id()` 返回实际回拨量对应的 `RetryAfter`，而不是继续分配序列。

随后覆盖：

- `GenerationOutcome::map` 对成功值和等待结果的行为；
- 不实现 `Display` 的自定义 ID 可以实现 `IdGenerator` 并格式化；
- 三种 Snowflake 在 `Immediate` 下首次调用立即生成；
- 两个全新同配置实例在固定同片时间下能够复现 `Immediate` 重复风险；
- 三种 Snowflake 在 `WaitNextSlice` 下首次调用等待、同片继续等待、跨片后
  从序列零开始；
- `try_next_id()` 在序列耗尽时立即返回边界等待时长，不调用 sleeper；
- `next_id()` 使用手动 sleeper 推进时间并完成生成，不发生真实休眠；
- Qubit 容忍范围内和超出范围的原始回拨；
- Classic 与 Sonyflake 在同一量化时间片内的原始回拨也会报错；
- sleeper 错误保留为 `SleepFailed` 的 source；
- 现有 compose/decode、边界、并发唯一性和 UUID-like 行为继续通过。

验证至少包括格式化、全量测试、Clippy、rustdoc/doctest、项目 style check 和
CI check。所有时间相关测试都必须确定性完成，不允许使用真实 `sleep` 来碰运气。

## 兼容性与发布影响

这是源代码 API 的破坏性重构，主要包括 trait 关联类型、builder 的 rs-clock
依赖注入、Qubit skew 的 `Duration` API、`IdError` 字段以及新增非阻塞结果
类型。调用方需要重新编译并调整接口使用。

生成 ID 的位布局、默认 epoch/start time、默认 Qubit 三秒回拨容忍、节点字段
和序列字段保持不变。默认重启策略为 `Immediate`，因此除原始时钟回拨判断更
准确之外，默认启动和分配行为不主动增加等待。

## 后续议题

完成本设计、通过验证并更新基准后，再次单独评估 mutex 临界区对单机吞吐的
限制。该评估应比较当前共享 mutex、批量序列租约和原子状态机方案，但不属于
本轮实现范围。
