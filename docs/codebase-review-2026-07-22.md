# 全仓 Code Review 与破坏性修复设计

> 文档类型：当前实现审查与目标态设计
> 审查基线：`34bbdbc`（`main`）
> 审查日期：2026-07-22
> 状态：已在 `agent/harden-runtime-control` 实施并完成自动及受控实机验证
> 兼容策略：允许修改或删除现有 Rust API、C ABI、bridge JSON 和 CLI 行为；不提供兼容层

## 结论

当前仓库的 crate 职责、S260 协议隔离、隐私投影、写后验证和发布质量门总体清晰，自动质量门也能覆盖主要编译边界。但运行时仍有几处会直接影响真实用户的缺陷，其中音量写入已经出现实机报错，应作为第一优先级处理。

目标不是在现有接口旁增加补丁，而是趁 Rust crates 尚未发布到 crates.io、公开 API 仍处于 `0.1` alpha，直接收紧语义：

1. 所有状态修改共用有截止时间的写后验证，不再只查询一次；
2. 所有超时分别表达连接、单次请求和写后验证，不再使用含义混杂的 `timeout`；
3. facade 只向产品入口返回已支持设备，未知候选只能从低层 discovery API 获取；
4. 事件 API 自己承担等待和退避，不把忙等责任泄漏给调用方；
5. 畸形响应可以按帧恢复，错误不得携带完整厂商响应；
6. macOS 所有设备操作串行执行，轮询不能与用户写入或滑块编辑竞争；
7. 删除只为假设中的第二型号存在的注册表和公开 S260 内部类型，等真实差异出现后再增加。

## 审查范围与验证

本次审查覆盖：

- `open-edifier-core` 公共类型、错误与同步契约；
- AA EC/BB EC 和 framed JSON decoder；
- mDNS 发现、型号识别与 facade 驱动选择；
- S260 状态解析、控制命令、写后验证和事件重连；
- CLI、Swift C ABI bridge 与 macOS SwiftUI MVP；
- mock 测试、CI、release workflow、Markdown 和 crate package 边界。

已执行并通过：

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo package --workspace --allow-dirty --no-verify
swiftc -typecheck -warnings-as-errors
Markdown 相对链接检查
```

Rust 共 23 个测试通过。此次审查没有重新打包 DMG，也没有操作实体音箱。

## 当前缺陷

### P1：音量写后验证会产生时序性假失败

`Client::set_volume` 当前流程是：

```text
读取范围 -> 写入音量 -> 等待 ACK -> 立即查询一次 -> 精确比较
```

该实现只证明了音量 `8 → 9 → 8` 在一次发布候选实机验收中可以立即读回，不能证明设备在所有时刻都会同步更新状态。若 ACK 已返回但状态投影稍晚更新，SDK 会报告 `Verification`，即使音箱随后已经达到目标音量。

同一风险也存在于输入源和 EQ。它们不应各自增加临时 sleep，而应共用一个有截止时间的验证循环。

测试目前没有直接覆盖 `set_volume`，也没有覆盖“第一次读回旧值、第二次读回目标值”和“截止时间内始终未达到目标值”。

### P1：macOS 轮询与用户音量操作存在竞争

macOS 每 5 秒发起一次静默状态刷新。静默刷新没有占用 `busy`，因此可能发生：

```text
用户开始拖动滑块
        |
        +-- 静默刷新返回旧音量并覆盖 volumeLevel
        |
用户结束拖动，提交被覆盖后的值
```

静默刷新还可能与手动写入建立两个并行 JSON 控制连接。当前没有双控制连接 mock，也没有实机证据证明 S260 在该并发模式下稳定。

### P1：名称可以把未知型号误识别为 S260

discovery 在硬件型号不是 `EDF100122` 时，仍会因为可修改的 AirPlay 实例名称包含 `s260` 而选择 S260 驱动。这会把未经验证的设备连接到 S260 的 `8080` 控制协议，违反型号支持边界。

型号映射只能使用经过验证的硬件字段。名称只能用于展示和人工选择，不能决定驱动。

### P1：事件流断线退避期间会忙等

`EventStream::next_event` 在重连时间未到时立即返回 `None`，CLI 收到 `None` 后立即继续循环。音箱断线时，调用方会在最长 5 秒的退避窗口内持续占用 CPU。

当前重连测试在测试代码里主动 sleep，因此没有覆盖真实 CLI 的忙等行为。读取错误和持续重连失败也被转成无限 `None`，调用方无法区分“暂时没有事件”和“连接已经长期失败”。

### P2：macOS 自动选择可能被未知设备阻断

底层 discovery 会保留未知 EDIFIER 候选，但 macOS 直接选择上次设备或列表第一项，不过滤已安装驱动。如果未知设备排在 S260 前面，首次状态读取会失败，App 也不会先展示列表让用户改选可支持设备。

### P2：畸形 JSON 帧会永久污染连接

framed JSON decoder 只在 JSON 解析成功后移除完整帧。解析失败时，坏帧留在 buffer 中；后续每次 feed 都会再次解析同一帧，当前 `Client` 无法恢复。

### P2：拒绝响应可能泄露完整厂商载荷

设备返回非零 code 或非 `success` message 时，当前错误保存完整 JSON 字符串。异常设备或伪造端点可以借此把 Wi-Fi、配对记录或任意字段带入 CLI stderr 和 macOS 错误提示。

### P2：状态投影缺少字段间约束

当前只校验数值能否放入 `u8`，没有拒绝以下状态：

- `minVolume > maxVolume`；
- `volume < minVolume` 或 `volume > maxVolume`；
- `selectedIndex >= soundIndex`；
- 音量事件中的 `current > max`。

这些不变量应在 wire model 投影到公共类型前校验。

### P3：超时不是严格上限

当前 `ClientConfig.timeout` 同时承担连接和请求语义，socket read timeout 又固定为 500ms。较短的调用方 timeout 仍可能被一次 read 越过；极大的 `Duration` 还可能在 `Instant + timeout` 时溢出。

## 理想目标态

### 1. 直接发布不兼容的 `0.2.0-alpha.1`

不保留 deprecated API，不增加 feature flag，不提供旧 C ABI wrapper，也不维持旧 bridge JSON 错误结构。仓库内消费者在同一个变更中一次迁移完成。

以下公共变化允许直接发生：

- 修改 `ClientConfig` 字段；
- 修改 `DeviceEvents` 方法签名；
- 收紧 facade `discover` 的结果语义；
- 删除 `open-edifier-s260::SpeakerStatus` 和协议 decoder 的公开导出；
- 修改 bridge JSON error schema；
- 删除单条静态驱动注册表及其假第二型号测试。

### 2. 重做超时和验证配置

将含义混杂的 `ClientConfig.timeout` 替换为明确字段：

```rust
pub struct ClientConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub verification_timeout: Duration,
    pub verification_interval: Duration,
}
```

约束：

- 四个 duration 都必须大于零；
- `verification_interval <= verification_timeout`；
- 所有 deadline 使用 `Instant::checked_add`；
- 每次 socket read 使用 `min(剩余时间, 500ms)`，保证请求 timeout 是真实上限；
- 默认验证窗口和间隔必须来自 S260 实机测量，并保留为硬件校准参数，不能写死在验证循环内部。

### 3. 所有修改共用一个验证循环

在 S260 driver 内实现一个私有 helper，输入截止时间和状态谓词：

```text
写入并收到 ACK
  -> 查询状态
  -> 已达到目标：成功
  -> 未达到且仍有时间：等待 verification_interval 后重试
  -> 到达截止时间：返回结构化 VerificationTimeout
```

新的错误必须包含：

- 修改字段；
- 请求值；
- 最后观察值；
- 查询次数；
- 实际等待时间。

输入源、音量和 EQ 全部复用该 helper。播放命令仍只保证 ACK，不进入状态验证循环。

禁止用以下方式修复：

- ACK 后固定 sleep 一次；
- 只给音量增加重试；
- 验证失败后返回成功并让 UI 猜测；
- 无上限重试。

### 4. 收紧 discovery 与产品入口边界

保留两个层次，但让语义不可混淆：

- `open-edifier-discovery::discover_candidates`：返回所有可识别的 EDIFIER 广播，包括未知型号；
- `open_edifier::discover`：只返回当前 build 已安装驱动的设备，供 CLI、C ABI 和应用使用。

删除基于名称的 S260 fallback。S260 只在硬件型号精确匹配 `EDF100122` 时映射到 `ModelId("s260")`。

CLI 和 macOS 不再各自实现 supported filtering。需要研究未知型号时，开发者直接使用低层 discovery crate；首版不增加 `--all` 产品选项。

### 5. 删除单型号注册表

当前静态驱动表只有 S260，一条注册记录和“第二型号”测试没有提供现实能力。facade 改为直接、穷尽的 model match：

```text
s260 -> S260 driver
其他 -> UnsupportedModel
```

保留 `Device`、`DeviceEvents`、`DeviceStatus` 等型号无关契约，因为 CLI、应用和未来驱动已经真实消费这些边界。等第二个型号具备硬件、固件、fixture/mock 和实机证据时，再决定 match 是否值得替换为表；不提前恢复插件或注册机制。

### 6. 重做事件等待契约

将：

```rust
fn next_event(&mut self) -> Result<Option<DeviceEvent>>;
```

改为：

```rust
fn next_event(&mut self, max_wait: Duration) -> Result<Option<DeviceEvent>>;
```

契约要求：

- 没有事件时至少等待到 `max_wait`、socket read 截止或下一次重连时间中的最早者；
- 退避窗口内不得立即返回形成忙等；
- 调用方可以用有限 `max_wait` 周期检查取消条件；
- 短暂断线可内部重连；
- 达到一次调用的等待预算仍无法重连时返回带最后错误原因的结构化网络错误，而不是永久返回 `None`；
- 成功重连后重置 decoder 和退避。

CLI 不再包含人为 sleep；等待语义由共享事件实现保证。

### 7. 让 framed JSON decoder 按帧恢复

完整 frame 一旦按 header 和长度切出，就必须先从内部 buffer 移除，再解析 JSON。解析失败返回当前帧错误，但不能阻止下一次 feed 处理后续有效帧。

decoder 保持私有，不再从 `open-edifier-s260` 公开导出。公开 API 只暴露设备状态、命令和事件，不暴露 wire transport。

`SpeakerStatus` 同样改为 crate 私有的 S260 wire 投影，其中保留写入输入源所需的 `input_index`。`Client` 的公开状态和修改方法直接返回 `open_edifier_core::DeviceStatus`，不再向调用方暴露第二套状态模型或厂商输入组字段。

### 8. 错误改为结构化、隐私安全的数据

bridge response 统一为：

```json
{
  "ok": false,
  "error": {
    "kind": "verification_timeout",
    "message": "音量未在验证窗口内达到目标值",
    "field": "volume",
    "expected": "18",
    "actual": "17"
  }
}
```

边界要求：

- `kind` 稳定，供 Swift UI 决定展示和恢复行为；
- `message` 面向用户，不包含完整响应；
- 只允许显式白名单字段进入错误；
- device `message` 截断到合理长度，并过滤控制字符；
- 未知 JSON 字段和私有状态永远不进入公开 error。

Swift bridge 不再把所有 Rust 错误压成一个字符串。

### 9. 串行化 macOS 控制

`SpeakerStore` 明确运行在主 actor，所有 blocking bridge 调用进入一个串行后台执行器。任意时刻只允许一个 discovery、status 或 mutation 与设备交互。

轮询规则：

- 滑块开始编辑时暂停静默刷新；
- 滑块结束后提交一次最终整数值；
- 写入完成并拿到验证状态后再恢复轮询；
- 用户操作排在静默刷新之前；
- 已排队但尚未开始的静默刷新可以丢弃；
- 失败时用最后一次已确认状态回滚滑块，而不是保留未确认值；
- 设备切换会取消旧设备尚未开始的操作。

不引入第三方响应式框架，不建立 daemon，不让 macOS 依赖 CLI。

### 10. 强化状态校验

`SpeakerStatus::from_value` 在构造任何公共状态前校验：

```text
min_volume <= volume <= max_volume
preset_count > 0（存在 EQ 状态时）
preset < preset_count
source index 属于 S260 已验证集合
```

事件流校验 `current <= max`。校验有效但语义未知的命令继续投影为 `DeviceEvent::Unknown`；已知命令但载荷不满足已验证结构时，不得伪装为合法状态事件。

## 文件级改动计划

| 文件 | 目标改动 |
|---|---|
| `crates/open-edifier-core/src/lib.rs` | 修改事件等待签名；增加结构化 verification/network 错误；删除不再需要的字符串错误形态 |
| `crates/open-edifier-discovery/src/lib.rs` | 重命名为候选发现；删除名称识别 S260；增加误命名未知型号测试 |
| `crates/open-edifier/src/lib.rs` | facade 过滤支持设备；单型号直接 match；删除静态注册表和假第二型号测试 |
| `crates/open-edifier-s260/src/client.rs` | 拆分 timeout；严格 deadline；共享写后验证循环 |
| `crates/open-edifier-s260/src/protocol.rs` | 先消费完整帧再解析；坏 JSON 后恢复 |
| `crates/open-edifier-s260/src/model.rs` | 校验音量范围、EQ 范围和公共状态不变量 |
| `crates/open-edifier-s260/src/events.rs` | 有等待预算的重连；禁止忙等；保留错误上下文；事件数值校验 |
| `crates/open-edifier-s260/src/lib.rs` | 移除 `SpeakerStatus`、`FrameDecoder`、`FRAME_HEADER` 的公开导出 |
| `bindings/swift/native/src/lib.rs` | 采用结构化 error JSON；只调用 facade 的 supported discovery |
| `apps/macos/OpenEdifierSwiftUI.swift` | 主 actor 状态；串行后台执行；滑块编辑期间暂停轮询；失败回滚 |
| `crates/open-edifier-s260/tests/client_mock.rs` | 增加直接音量、延迟生效、验证超时、隐私错误和坏帧恢复测试 |
| `crates/open-edifier-s260/tests/events_mock.rs` | 增加断线等待耗时、失败原因和无忙等测试 |
| `README.md`、`CHANGELOG.md`、对应 docs | 同步新 API、错误语义、发现边界、超时和实机验证结果 |

## 实施顺序

### 阶段一：先固定失败测试

只增加能在当前实现上失败的最小测试：

1. 音量第一次读回旧值、第二次达到目标；
2. 音量在验证截止前始终不变；
3. 坏 JSON 后紧跟有效 JSON；
4. 未知型号名称含 `S260`；
5. rejected response 携带私有字段；
6. 事件断线后一次 `next_event(max_wait)` 不得立即返回；
7. macOS 轮询结果不得覆盖正在编辑的滑块。

### 阶段二：重构 Rust 契约和实现

一次性修改 core、discovery、facade 和 S260 driver。不要先增加兼容 API 再删除旧 API；直接让所有消费者编译失败，然后逐个迁移。

### 阶段三：迁移 CLI、bridge 和 macOS

CLI 使用新的发现和事件等待契约。bridge 使用结构化错误。macOS 完成串行操作和滑块编辑状态后，删除旧的 `busy + operationRevision` 竞态控制方案。

### 阶段四：文档与实机验证

自动质量门通过后再执行实机测试。测试前记录输入源、音量、EQ 和播放状态；仅使用相邻安全音量值，并在结束时恢复。

## 验收标准

### 自动测试

- 音量状态延迟至少一次查询后更新时，调用成功且返回最终状态；
- 音量始终不更新时，在固定截止时间内返回结构化 `verification_timeout`；
- source 和 EQ 使用同一验证机制；
- malformed JSON 不会毒化后续有效响应；
- unknown device 即使名为 `S260` 也不会加载 S260 驱动；
- rejected response 的私有字段不会出现在 Rust error、CLI stderr 或 bridge JSON；
- 事件断线退避期间没有紧循环；
- macOS 静默刷新不会覆盖用户正在编辑的音量；
- facade、CLI 和 macOS 永远不会自动选择未知型号。

### 质量门

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo package --workspace --allow-dirty --no-verify
apps/macos/package.sh
```

### S260 实机

至少验证：

1. 读取并记录初始输入源、音量、EQ 和播放状态；
2. 音量执行 `N → N+1 → N`，记录每次验证查询次数和总耗时；
3. 在一次音量拖动跨越 5 秒轮询边界时，最终只提交用户选定值；
4. 连续执行十次相邻安全音量修改，不出现假 `VerificationTimeout`；
5. 模拟应用侧取消/重连，不重启路由器、不暴露端口；
6. 恢复初始音量、EQ 和可用输入源；
7. 记录已验证项、未验证项和设备最终状态。

## 明确不做

- 不为旧 Rust API、旧 C ABI 或旧 bridge JSON 保留 adapter；
- 不增加 async runtime crate；
- 不增加第三方 retry、actor 或状态管理依赖；
- 不为尚未出现的第二型号恢复注册表、插件或工厂；
- 不把写后验证降级为 ACK 成功；
- 不通过扩大实机修改幅度来“提高测试可信度”；
- 不执行恢复出厂、固件升级、关机、重命名或网络破坏性测试。

## 完成定义

只有以下条件同时满足，才认为这轮破坏性修复完成：

- 本文列出的 P1/P2 问题都有会在旧实现失败的回归测试；
- Rust、CLI、bridge 和 macOS 已全部迁移到新契约，仓库中不存在兼容 shim；
- 文档不再描述单次立即查询、名称型号识别或调用方忙等；
- 自动质量门、macOS 打包和受控 S260 实机测试通过；
- 音箱最终状态恢复并记录；
- 版本提升到 `0.2.0-alpha.1`，`CHANGELOG.md` 明确列出不兼容变化。

## 实施结果

本方案已按不兼容策略实施：旧 Rust API、事件签名和 bridge error JSON 已直接删除，没有兼容 shim。自动验证、macOS DMG 打包和 S260 实机测试均通过；实机最终恢复为测试前的 AirPlay、音量 `10`、EQ `0`、停止。候选的完整验证记录见 [`0.2.0-alpha.1` release notes](release-notes/0.2.0-alpha.1.md)。
