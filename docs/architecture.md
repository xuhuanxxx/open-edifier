# 架构设计

OpenEdifier 是面向多型号的开源 monorepo。项目整体是产品；Rust SDK 是仓库内唯一共享能力层，CLI、iOS、macOS 和 Home Assistant 是相互平行的使用入口或集成。

```text
CLI -----------------------------> Rust SDK --\
macOS -> C ABI bridge -----------> Rust SDK ---+-> 驱动选择 -> 型号驱动 -> 音箱
iOS -> Swift binding（规划）-----> Rust SDK ---+
Home Assistant -> Python binding -> Rust SDK --/
```

## 项目边界

- SDK 提供发现、状态、命令、事件和结构化错误，是唯一共享控制能力。
- CLI 直接调用 Rust SDK，是终端用户和本地 Agent 的主要自动化入口。
- macOS 应用通过最小 C ABI bridge 静态链接 SDK，不要求 CLI 或任何 daemon 运行。
- iOS 计划通过 Swift binding 直接调用 SDK。
- Home Assistant 通过 Python binding 或专用 integration 调用 SDK。
- 任一使用入口未安装、未启动或发生故障，都不能阻断其他入口使用共享能力层。

公共纯前端 WebUI 不属于当前产品架构。浏览器无法直接访问 S260 的原始 TCP/mDNS 控制面，Rust/Wasm 也不会绕过浏览器沙箱；该方向的论证已移入 [归档](archive/webui-plan.md)。

## 驱动层

经过验证的 S260 驱动在 TCP `8080` 端口使用两条独立传输路径：framed JSON 用于完整状态和带确认的设置，`BB EC` 二进制通道用于低延迟状态事件。所有使用入口通过与型号无关的状态和事件类型使用两者，不直接解析任何 wire format。

`open-edifier-core` 只拥有稳定、与型号无关的类型和同步 `Device` 契约。型号与输入源标识使用字符串承载，使驱动能够增加能力，而无需修改中央 enum。

`open-edifier-discovery` 通过设备公开的局域网广播查找 EDIFIER 设备。发现层只识别候选设备，不会把广播的媒体端口当作控制端口。

控制端点归驱动所有。发现层返回设备身份和网络地址，由选中的型号驱动提供经过验证的默认控制端口，避免在发现结果中重复保存协议配置。

每个 Rust 驱动负责一个协议族的分帧、型号特定校验、控制端点和设备命令。`open-edifier` facade 负责选择驱动；应用层负责 UI、生命周期、局域网权限和平台集成。

语言绑定必须保留各驱动的状态和错误语义。应用不应复制 wire protocol。

S260 wire response 会解析为私有 Serde 传输类型。公开状态只包含应用需要的稳定字段，不保留可能携带局域网或已配对设备信息的完整厂商响应。

## 新型号扩展准备度

当前架构已经完成第二型号所需的最小代码重构，但尚未经过第二台真实硬件验证，因此不能把任何其他型号声明为已支持。

已经具备的扩展点：

- `ModelId`、`Source` 和 `PlaybackState` 使用字符串承载，新标识不需要修改中央 enum；
- `Device` 和 `DeviceEvents` 提供型号无关的同步控制及事件契约；输入源、音量、EQ 和播放修改都有默认的结构化能力错误；
- `DeviceStatus` 允许输入源、音量、EQ 和播放状态缺失，并通过稳定的 `DeviceCapabilities` 公布可用输入源及修改能力；
- 未知但校验有效的事件仍以 `DeviceEvent::Unknown` 保持可观察；
- discovery 会保留未知 EDIFIER 型号，只有已注册驱动的型号才能自动选择和控制；
- 控制端口、wire model、命令语义和写后验证归型号驱动所有；
- `open-edifier-aaec` 只复用帧结构，不拥有 S260 子命令或载荷语义；
- facade 通过一个静态驱动注册表集中保存型号、默认端口、控制连接和可选事件连接；CLI 和 C ABI 不包含型号分支；
- macOS App 根据 `DeviceCapabilities` 动态显示输入源、音量、EQ 和播放控件。

新增一个型号的预期改动范围：

1. 记录型号、固件、发现方式、控制端点和命令证据；
2. 新建独立型号驱动 crate，实现适用的 `Device` 和 `DeviceEvents`；
3. 在 discovery 中增加经过验证的型号识别映射；
4. 在 facade 的静态表中增加一条驱动、默认端口和可选事件连接注册；
5. 补充脱敏 fixture、确定性 mock、写后验证和实机恢复记录；
6. 让 CLI 和应用根据公共能力显示功能，不在应用层复制型号协议。

当前仍保留一个证据驱动的边界：discovery 只浏览已经由 S260 实机确认的 AirPlay mDNS 服务。新型号若使用同一广播，只需增加型号代码映射；若使用不同发现方式，再增加对应 discoverer，不提前构造通用发现插件。

单元测试已经覆盖无输入源/音量的最小 `Device`、未知型号、未知事件和包含两个型号条目的驱动表选择。第二型号出现前，不再引入动态插件、通用传输框架、宏注册表或 async runtime。
