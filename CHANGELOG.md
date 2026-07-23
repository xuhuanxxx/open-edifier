# 变更记录

## 0.2.0-alpha.1 - 尚未发布

- 不兼容地拆分 S260 连接、请求和写后验证超时，并为输入源、音量和 EQ 共用有截止时间的验证循环。
- `Client` 直接返回公共 `DeviceStatus`；S260 内部状态和 framed JSON decoder 不再公开。
- `DeviceEvents::next_event` 增加调用方等待预算，修复断线退避期间的 CLI 忙等并保留重连错误。
- facade 发现只返回当前 build 支持的设备；低层 `discover_candidates` 保留未知候选，型号映射不再信任可修改名称。
- framed JSON decoder 可在畸形帧后恢复，拒绝错误不再携带完整设备响应，状态和事件增加字段间校验。
- Swift bridge 使用结构化错误；macOS 串行执行设备操作，在音量滑块编辑期间暂停轮询并在失败时回滚。
- 版本提升到 `0.2.0-alpha.1`，不提供旧 Rust API、事件签名或 bridge error JSON 的兼容层。
- 新增可 pip 构建的纯 Python 异步 S260 客户端，零运行时依赖，提供状态、写后验证、播放、结构化错误和可重连事件流；Home Assistant integration 仍未交付。

## 0.1.0-alpha.1 - 2026-07-20

- 新增与型号无关的核心契约和高层驱动选择。
- 使用 EDIFIER AirPlay 广播实现跨平台 mDNS 发现。
- 新增零配置 CLI 命令和 JSON 输出。
- 新增首个经过验证的 S260 状态、输入源和音量驱动。
- 新增协议分帧与模拟设备测试。
- 使用广播的硬件型号识别被用户重命名的 S260。
- 通过共享 SDK 保留结构化驱动错误。
- 直接连接主机时强制显式选择型号。
- 新增有界连接尝试和严格响应校验。
- 新增跨平台 CI、MSRV 检查、安全指南和 crate 文件边界检查。
- 新增经过验证的 S260 EQ 与播放控制。
- 新增带校验和验证的可复用 AA EC/BB EC 分帧。
- 新增带类型的实时输入源、音量、播放、EQ 和未知事件。
- 统一并记录最终确认的一字节二进制载荷长度格式。
- 新增脱敏的逆向研究纪实，并致谢同族协议的先行研究。
- 使用私有强类型 wire model 解析 S260 状态，避免在公开序列化中携带完整厂商响应。
- 从 S260 公开 API 中移除未经验证的 XOR 传输选项。
- 支持从损坏的二进制候选帧中恢复，并使用有界退避重连事件流。
- 控制端口只由驱动 crate 定义，不再存入发现结果。
- 明确 OpenEdifier 开源项目整体是产品，CLI、macOS 和规划中的集成是共享 Rust 能力层之上的平行使用入口。
- 归档公共纯前端 WebUI 方案：浏览器无法直连 S260 的原始 TCP/mDNS 控制面，本项目不以隐含本地服务替代纯前端目标。
- 新增可运行的原生 SwiftUI macOS MVP，通过最小 C ABI 静态链接 Rust SDK，支持发现、状态、输入源、音量、EQ 和播放控制。
- macOS 应用使用 `WindowGroup` 接入标准系统菜单和窗口管理，并支持设备选择记忆、静默状态同步、控制菜单及键盘快捷键。
- 新增 CLI 版本输出、macOS bundle 版本注入、原创可复现图标、DMG/checksum 打包和 tag 驱动的 prerelease workflow。
- 完成多型号最小重构：core 移除 S260 常量，公共状态允许可选能力，新增稳定能力投影和集中式静态驱动注册，macOS 按能力动态显示控制项。
- 建立 public GitHub 仓库、私密漏洞报告入口、Homebrew tap 和带 DMG/checksum 的首个 GitHub prerelease。
