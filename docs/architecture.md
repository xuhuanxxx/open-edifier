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

当前架构已经为第二个真实型号提供了足够的最小接缝，但尚未经过第二型号验证，因此不能称为任意型号即插即用。

已经具备的扩展点：

- `ModelId`、`Source` 和 `PlaybackState` 使用字符串承载，新标识不需要修改中央 enum；
- `Device` 和 `DeviceEvents` 提供型号无关的同步控制及事件契约；
- EQ 和播放状态允许缺失，不支持的命令返回结构化能力错误；
- 未知但校验有效的事件仍以 `DeviceEvent::Unknown` 保持可观察；
- discovery 会保留未知 EDIFIER 型号，只有已注册驱动的型号才能自动选择和控制；
- 控制端口、wire model、命令语义和写后验证归型号驱动所有；
- `open-edifier-aaec` 只复用帧结构，不拥有 S260 子命令或载荷语义；
- facade、CLI 和 C ABI 使用 `ModelId` 选择驱动，不要求应用解析 wire protocol。

新增一个型号的预期改动范围：

1. 记录型号、固件、发现方式、控制端点和命令证据；
2. 新建独立型号驱动 crate，实现适用的 `Device` 和 `DeviceEvents`；
3. 在 discovery 中增加经过验证的型号识别映射；
4. 在 facade 中注册驱动、默认端口和事件连接；
5. 补充脱敏 fixture、确定性 mock、写后验证和实机恢复记录；
6. 让 CLI 和应用根据公共能力显示功能，不在应用层复制型号协议。

当前已知但不阻塞第二型号的假设：

- `open-edifier-core` 仍包含 `MODEL_S260` 常量，存在轻微型号泄漏；
- facade 的控制和事件入口分别手工匹配驱动，型号增多后可能产生重复；
- `DeviceStatus` 强制包含输入源和音量，`Device` 也要求对应设置方法；
- discovery 当前只浏览已验证的 AirPlay mDNS 服务；
- macOS UI 当前写死蓝牙、AUX、USB、AirPlay 和播放控件，没有完整消费 `capabilities`。

如果第二型号沿用 AirPlay 发现并具备基本输入源和音量控制，现有结构足以直接增加驱动。如果真实型号缺少这些能力、使用不同发现方式或暴露新的状态形态，再根据证据调整公共契约。第二型号出现前，不为上述假设引入插件系统、通用传输框架、宏注册表或 async runtime。
