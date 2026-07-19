# OpenEdifier

为受支持的 EDIFIER 音箱提供非官方、本地优先的控制能力。

OpenEdifier 目前提供 Rust SDK、CLI 和原生 macOS MVP，可在可信局域网内发现并控制 EDIFIER S260。iOS、公开语言绑定和 Home Assistant 支持仍在规划中，不包含在当前 alpha 版本内。

## 已实现功能

| 型号 | 已测试固件 | 发现 | 状态 | 输入源 | 音量 | EQ | 播放控制 | 实时事件 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| S260 | `01.00.00` | 是 | 是 | 是 | 是 | 是 | 是 | 是 |

已验证的 S260 输入源包括蓝牙、AUX、USB 和 AirPlay。修改输入源、音量和 EQ 后，SDK 会再次查询状态；只有在设备端观察到目标状态才会报告成功。播放命令由音箱确认接收，但实际效果取决于当前输入源。

## 从源码安装

需要 Rust 1.85 或更高版本。

```bash
cargo install --path crates/open-edifier-cli --locked
```

随后无需指定 IP 地址即可发现并控制音箱：

```bash
edifier --version
edifier discover
edifier status
edifier source aux
edifier source usb
edifier source airplay
edifier volume 18
edifier eq
edifier eq 1
edifier play
edifier pause
edifier next
edifier prev
edifier listen
edifier --json status
edifier --json listen --count 2
```

存在多台受支持的音箱时，可使用 `discover` 输出的 ID、名称或主机名选择设备：

```bash
edifier --device "Living Room" status
```

绕过自动发现时必须显式指定型号。端口默认使用对应驱动已验证的值：

```bash
edifier --host 192.0.2.10 --model s260 status
edifier --host 192.0.2.10 --model s260 --port 8080 status
```

`--json` 为脚本和本地 Agent 输出机器可读的发现、状态与事件数据。错误写入 stderr，并使用非零退出码。

## Monorepo 结构

```text
crates/
  open-edifier/            高层 SDK 与驱动选择
  open-edifier-aaec/       可复用的 AA EC/BB EC 二进制分帧
  open-edifier-core/       与型号无关的公共契约
  open-edifier-discovery/  mDNS 设备发现
  open-edifier-s260/       已验证的 S260 协议驱动
  open-edifier-cli/        多型号 CLI
apps/                      平行的用户端应用；macOS MVP 已交付，iOS 尚在规划
bindings/                  macOS C ABI bridge 与规划中的公开 Swift/Python 绑定
integrations/              规划中的 Home Assistant 集成
docs/                      架构与协议文档
research/                  仅存放脱敏后的原始观察
```

## macOS 应用

Apple Silicon、macOS 26 本机构建：

```bash
apps/macos/build.sh
open apps/macos/dist/OpenEdifier.app
```

生成 DMG 和 checksum：

```bash
apps/macos/package.sh
```

应用会自动发现音箱，并提供输入源、音量、EQ 和播放控制。当前产物只支持 Apple Silicon 并使用 ad-hoc 签名；Developer ID 和公证不是首发前提。Intel 和更低 macOS 版本支持由真实用户需求触发。详见 [macOS 应用说明](apps/macos/README.md)。

## Alpha 限制

- 目前只验证了固件 `01.00.00` 的 S260。
- 自动发现依赖设备的 AirPlay 广播；仍可直接指定主机连接。
- 实时事件使用单独观测到的 `BB EC` 推送通道；遇到损坏候选帧会恢复同步，短暂断线后会采用有界退避重连。未知事件命令会以带类型的原始载荷向上暴露。
- S260 控制协议没有认证和加密。只能在可信局域网内使用，绝不能将 `8080` 端口暴露到互联网。
- `1.0` 之前，公开 Rust API 和 JSON 字段可能调整。
- 恢复出厂、固件升级、重命名、关机等破坏性命令被有意排除。

进一步阅读：[协议说明](docs/protocol.md)、[架构设计](docs/architecture.md)、[产品化路线](docs/productization.md)、[发布操作](docs/release.md)、[逆向研究过程](docs/research-journey.md)、[安全政策](SECURITY.md)和[贡献指南](CONTRIBUTING.md)。

## 项目边界

产品是 OpenEdifier 这个公开开源项目整体。Rust SDK 是仓库内唯一共享的设备能力层；CLI、macOS、规划中的 iOS 和 Home Assistant 是项目提供的平行使用入口，不依赖另一个入口运行。macOS 应用通过最小 C ABI 静态链接 SDK，不调用 CLI。

公共纯前端 WebUI 方向已经归档：普通浏览器不能访问 S260 使用的原始 TCP socket 和 mDNS，而引入本地 companion 又不符合纯前端目标。完整论证保留在 [WebUI 纯前端方案归档](docs/archive/webui-plan.md)。

## 路线图

- 按[产品化路线](docs/productization.md)维护公开审计、可追踪发布和最小社区治理
- 通过项目 Homebrew tap 发布 CLI，稳定后再评估 `homebrew-core`
- 将 ad-hoc 签名的 macOS App 通过 GitHub Release 分发；Developer ID 和 Apple 公证作为可选升级
- 支持 App Intents 的原生 iOS 应用
- Python 绑定与 Home Assistant 集成
- 以脱敏 fixture 或实机验证为依据，支持更多 EDIFIER 型号

## 致谢

首个 S260 驱动参考了 [edifier-es300](https://github.com/rabbit-aaron/edifier-es300)、[esphome-edifier-d32](https://github.com/rabbit-aaron/esphome-edifier-d32) 和 [edf-controller](https://github.com/rioriost/edf-controller) 对同族协议的公开研究。OpenEdifier 会独立验证每个型号的具体行为，确认后才声明支持。

## 法律声明

OpenEdifier 是独立的社区项目，与 EDIFIER 无附属、认可或赞助关系。EDIFIER 及相关产品名称是其各自所有者的商标。

本项目采用 MIT 许可证。
