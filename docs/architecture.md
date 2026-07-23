# 架构设计

OpenEdifier 是面向多型号的开源 monorepo。项目整体是产品；Rust SDK 是原生产品的共享能力层，纯 Python 客户端是面向 Home Assistant 安装与异步运行时的独立实现。CLI、iOS、macOS 和 Home Assistant 是相互平行的使用入口或集成。

```text
CLI --------------------------> Rust SDK --\
macOS -> C ABI bridge --------> Rust SDK ---+-> Rust 型号驱动 -----> 音箱
iOS -> Swift binding（规划）--> Rust SDK --/
Home Assistant（规划）--------> async Python client -> Python S260 驱动 -> 音箱
```

## 项目边界

- Rust SDK 提供原生端的发现、状态、命令、事件和结构化错误。
- 纯 Python 客户端直接提供异步状态、命令和事件，不调用 CLI、动态库或 Rust 构建产物；Home Assistant 发现由平台 Zeroconf 提供。
- CLI 直接调用 Rust SDK，是终端用户和本地 Agent 的主要自动化入口。
- macOS 应用通过最小 C ABI bridge 静态链接 SDK，不要求 CLI 或任何 daemon 运行。
- iOS 计划通过 Swift binding 直接调用 SDK。
- Home Assistant 计划通过专用 integration 调用纯 Python 客户端。
- 任一使用入口未安装、未启动或发生故障，都不能阻断其他入口。

公共纯前端 WebUI 不属于当前产品架构。浏览器无法直接访问 S260 的原始 TCP/mDNS 控制面，Rust/Wasm 也不会绕过浏览器沙箱；该方向的论证已移入 [归档](archive/webui-plan.md)。

## 驱动层

经过验证的 S260 驱动在 TCP `8080` 端口使用两条独立传输路径：framed JSON 用于完整状态和带确认的设置，`BB EC` 二进制通道用于低延迟状态事件。Rust 应用不直接解析 wire format；纯 Python 客户端独立实现同一已记录边界。

`open-edifier-core` 只拥有稳定、与型号无关的类型和同步 `Device` 契约。型号与输入源标识使用字符串承载，使驱动能够增加能力，而无需修改中央 enum。

`open-edifier-discovery` 通过设备公开的局域网广播查找 EDIFIER 候选设备，不会把广播的媒体端口当作控制端口。低层 `discover_candidates` 保留未知型号；高层 facade 的 `discover` 只返回当前 build 已支持的设备，产品入口不会重复实现型号过滤。

控制端点归驱动所有。发现层返回设备身份和网络地址，由选中的型号驱动提供经过验证的默认控制端口，避免在发现结果中重复保存协议配置。

每个 Rust 驱动负责一个协议族的分帧、型号特定校验、控制端点和设备命令。`open-edifier` facade 负责选择驱动；应用层负责 UI、生命周期、局域网权限和平台集成。

Swift binding 必须保留 Rust 驱动的状态和错误语义。Python 客户端保持零运行时依赖和原生 `asyncio`，并通过对应 mock 测试与 Rust 实现对齐；协议语义变化必须同时更新两端和协议文档。

S260 wire response 在 Rust 中解析为私有 Serde 传输类型，在 Python 中通过私有严格解析函数验证。公开状态只包含应用需要的稳定字段，不保留可能携带局域网或已配对设备信息的完整厂商响应。设备拒绝也只投影结果 code 和经过限制的 message，不把完整响应放入错误。

输入源、音量和 EQ 在 ACK 后共用有截止时间的验证循环，允许设备状态短暂延迟；播放命令仍只保证 ACK。事件读取由调用方提供最大等待时间，驱动在该预算内承担 socket read、退避和重连，不要求调用方忙等。

## 新型号扩展准备度

当前架构保留第二型号所需的公共契约，但尚未经过第二台真实硬件验证，因此不能把任何其他型号声明为已支持。

已经具备的扩展点：

- `ModelId`、`Source` 和 `PlaybackState` 使用字符串承载，新标识不需要修改中央 enum；
- `Device` 和 `DeviceEvents` 提供型号无关的同步控制及事件契约；输入源、音量、EQ 和播放修改都有默认的结构化能力错误；
- `DeviceStatus` 允许输入源、音量、EQ 和播放状态缺失，并通过稳定的 `DeviceCapabilities` 公布可用输入源及修改能力；
- 未知但校验有效的事件仍以 `DeviceEvent::Unknown` 保持可观察；
- 低层 discovery 会保留未知 EDIFIER 型号，facade 和产品入口只返回并控制已支持型号；
- 控制端口、wire model、命令语义和写后验证归型号驱动所有；
- `open-edifier-aaec` 只复用帧结构，不拥有 S260 子命令或载荷语义；
- facade 当前通过一个穷尽的 S260 model match 选择驱动；第二型号出现真实证据前不维护注册表；
- macOS App 根据 `DeviceCapabilities` 动态显示输入源、音量、EQ 和播放控件。

新增一个型号的预期改动范围：

1. 记录型号、固件、发现方式、控制端点和命令证据；
2. 新建独立型号驱动 crate，实现适用的 `Device` 和 `DeviceEvents`；
3. 在 discovery 中增加经过验证的型号识别映射；
4. 在 facade 的 model match 中接入驱动、默认端口和可选事件连接；第二型号落地后再判断是否值得改成静态表；
5. 补充脱敏 fixture、确定性 mock、写后验证和实机恢复记录；
6. 让 CLI 和应用根据公共能力显示功能，不在应用层复制型号协议。

当前仍保留一个证据驱动的边界：discovery 只浏览已经由 S260 实机确认的 AirPlay mDNS 服务。新型号若使用同一广播，只需增加型号代码映射；若使用不同发现方式，再增加对应 discoverer，不提前构造通用发现插件。

单元测试已经覆盖无输入源/音量的最小 `Device`、未知型号、未知事件、坏帧恢复、写后验证延迟和事件重连等待。第二型号出现前，不再引入静态注册表、动态插件、通用传输框架、宏注册表或 async runtime。
