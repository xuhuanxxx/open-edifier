# OpenEdifier 开源项目产品化路线

本文定义 OpenEdifier 从当前 alpha 代码基线走向公开开源项目的工作范围和验收标准。本文是规划，不表示其中列出的发布能力已经交付。

## 产品定义

产品只有一个：**OpenEdifier 开源项目本身**。

Rust workspace、CLI、macOS App、文档、测试和研究记录共同组成这个项目。它们在代码架构中承担不同职责，但不是产品化阶段需要分别经营和发布的多个产品：

- Rust SDK 是仓库内部的共享能力层，不单独发布到 crates.io；
- CLI 是项目提供的一种使用入口，Homebrew 是它的安装方式；
- macOS App 是项目提供的一种图形入口，GitHub Release 中的 DMG 是它的分发方式；
- iOS、公开语言绑定和 Home Assistant 是项目路线图，不是当前已经交付的能力。

产品化的目标是让一个不了解仓库历史的人来到公开 GitHub 后，能够：

1. 看懂项目解决什么问题、支持什么、不支持什么；
2. 在不接触私有资料的情况下构建、测试和使用当前能力；
3. 通过 Homebrew 安装 CLI，或构建、下载 macOS App；
4. 判断自己的设备是否在验证边界内，并安全地报告问题；
5. 复现 release 对应的源码、测试和安装结果；
6. 按明确证据和驱动边界贡献新型号，而不复制或污染现有协议实现。

因此，GitHub public 不是宣传渠道，而是产品的正式形态；Homebrew formula、DMG、tag 和 Release notes 都只是这个开源项目的发布产物。

## 首个公开版本的边界

下面是 `v0.1.0-alpha.1` 的验证边界，不是 OpenEdifier 的长期产品定义：

- 型号：EDIFIER S260；
- 已验证固件：`01.00.00`；
- 网络：可信局域网内的 mDNS 发现和 TCP 直连；
- 能力：发现、状态、输入源、音量、EQ、播放控制和实时事件；
- macOS：Apple Silicon、macOS 26；
- 隐私：无账号、无云端、无遥测，不记录厂商完整状态；
- 安全：不公开恢复出厂、固件升级、关机和重命名等破坏性能力；
- CLI 更新：使用 `brew upgrade`；
- macOS App 更新：首版由用户手动下载安装新 DMG。

没有型号、固件、脱敏 fixture、mock 或实机记录时，不得扩大支持声明。

## 当前状态

| 项目 | 当前状态 | 证据或剩余工作 |
|---|---|---|
| 核心能力 | 已完成 | S260 驱动、发现、CLI、SwiftUI App、mock 和架构文档均在仓库内 |
| 多型号基础 | 已完成代码重构 | core 无型号常量，状态能力可选且结构化，facade 集中注册驱动，CLI/C ABI 无型号分支，macOS 按能力渲染；仍只有 S260 经过实机验证 |
| Git 基线 | 已完成 | `main` 已建立初始 commit，并推送到个人 private GitHub 仓库 |
| 敏感信息与历史审计 | 当前已通过 | 已检查当前完整 Git 历史中的路径、地址、凭据特征、专有扩展名和大 blob，ignored 内容只有构建产物；转 public 前仍须对最终增量复跑 |
| 最小公开文件 | 已完成 | README、LICENSE、CONTRIBUTING、SECURITY、CHANGELOG 和 `.gitignore` 已存在 |
| Rust CI | 已完成并通过 | Linux、macOS、Windows 测试，fmt、clippy、doc 和 Rust 1.85 MSRV 已在 GitHub Actions 通过 |
| CLI 版本 | 已完成 | `edifier --version` 从 Cargo package version 输出完整版本 |
| macOS 发布工程 | 已完成并通过 | 版本注入、原创图标、ad-hoc 签名、DMG 和 checksum 已落地，本地与 GitHub Actions 构建均通过 |
| tag 到 GitHub prerelease | 两阶段流程已完成，尚未触发 | tag 推送只做候选验收；Homebrew 线上验证通过后人工触发同一 workflow 创建 prerelease；当前没有 tag |
| GitHub public | 未完成 | 当前仓库保持 private，改为 public 需要明确授权 |
| Homebrew tap | 配方本地预验完成，远端未创建 | 使用当前干净 commit 的源码归档和临时本地 tap，已通过 style、源码安装、test、version/help 和卸载；public tag URL、SHA-256、online audit 与升级仍需在仓库 public 后验证 |
| 发布候选实机验收 | 已完成 | S260 完成状态、最小音量/EQ 写入恢复、当前输入源、播放 ACK 和实时事件验证，最终状态已记录 |
| DMG 结构验收 | 已完成 | image checksum、挂载、App、`/Applications` 快捷方式和 bundle 签名已回读验证 |
| 干净环境安装 | 部分完成 | 全新 private clone 的 CLI 源码安装，以及临时本地 tap 的 Homebrew 安装、测试和卸载已通过；仍需 public tap、升级和带 quarantine 的非开发 macOS 用户环境验证 |

## 产品化原则

1. 产品化对象始终是整个公开仓库，不把内部 crate 或应用入口虚构成独立发布目标。
2. Rust SDK 继续作为唯一共享能力层，CLI 和应用不得复制 wire protocol。
3. 冻结 `v0.1` 设备能力，公开准备不顺带增加命令、新型号或推测性抽象。
4. 保持同步 Rust 核心，不引入 async runtime、daemon、通用传输框架或插件系统。
5. 所有发布产物必须来自同一干净 commit 和 tag，并能由公开源码重新构建。
6. 首发不要求付费 Apple Developer Program；Developer ID 和公证只是未来可选升级。
7. 不为跨显示器时的系统级闪烁增加应用 workaround；该现象不作为当前项目缺陷处理。
8. 已交付能力、验证边界和路线图必须分开表述。
9. 新工作必须改善项目的可理解、可使用、可验证、可维护或可贡献性，否则不属于当前产品化范围。

## P0：公开仓库准备

### 公开前审计

仓库当前只有 private 历史。转为 public 前必须重新检查所有文件；如果发现敏感内容，必须在改变可见性前清理 private 历史：

```bash
rg -n '/Users/|/home/|[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){5}' . \
  --glob '!target/**' --glob '!apps/macos/dist/**'
rg -n '\b(10\.|172\.(1[6-9]|2[0-9]|3[01])\.|192\.168\.)' . \
  --glob '!target/**' --glob '!apps/macos/dist/**'
rg --files | rg '\.(apk|dex|pcap|pcapng|bin|firmware|mobileprovision|p12)$'
```

必须人工确认：

- tracked、untracked 和 ignored 文件；
- workflow、脚本和本地配置中的 token、账号及绝对路径；
- fixture 中的设备名、主机名、Wi-Fi、MAC 和蓝牙配对信息；
- `target/`、`apps/macos/dist/` 等本地产物已忽略；
- 不存在 APK、DEX、固件、原始抓包、厂商手册或解包库；
- README 的商标声明、支持边界和第三方研究致谢准确。

审计通过后再创建初始 commit，避免敏感内容进入公开 Git 历史。

### 最小公开文件

首次 public 必须具备：

- `README.md`：用途、已验证型号、安装、示例、安全边界和路线图；
- `LICENSE`：明确开源许可；
- `CONTRIBUTING.md`：开发环境、质量门、新型号证据和实机测试规则；
- `SECURITY.md`：漏洞报告方式和明文局域网协议限制；
- `CHANGELOG.md`：当前版本和用户可见变化；
- `.gitignore`：构建产物、证书、凭据和本地配置不进入仓库。

Issue 模板、Pull Request 模板、`CODE_OF_CONDUCT.md`、ruleset、Dependabot 和 private vulnerability reporting 都有价值，但不是一个个人开源项目首次 public 的阻塞项。出现外部协作或维护噪音后再补，不为“看起来正规”预建流程。

GitHub 参考：[Community profile](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/about-community-profiles-for-public-repositories) 和 [Repository best practices](https://docs.github.com/en/enterprise-cloud@latest/repositories/creating-and-managing-repositories/best-practices-for-repositories)。

### CI

首次 public 只需要两个确定性检查：

1. Rust workspace 质量门；
2. macOS runner 执行 `apps/macos/package.sh`，验证构建、图标、ad-hoc 签名、DMG 和 checksum。

普通 CI 不需要证书、Apple 账号、设备地址或其他私密环境。

## P0：项目可安装和可使用

### Homebrew CLI

主仓库转为 public 并建立 release tag 后，创建独立 public tap：

```text
xuhuanxxx/homebrew-tap
  Formula/
    open-edifier.rb
```

用户安装：

```bash
brew install xuhuanxxx/tap/open-edifier
```

formula 名称使用 `open-edifier`，安装二进制保持 `edifier`。它只构建 CLI，不构建 macOS App。

```ruby
class OpenEdifier < Formula
  desc "Control supported EDIFIER speakers over a local network"
  homepage "https://github.com/xuhuanxxx/open-edifier"
  url "https://github.com/xuhuanxxx/open-edifier/archive/refs/tags/v<version>.tar.gz"
  sha256 "<sha256>"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/open-edifier-cli")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/edifier --version")
  end
end
```

发布前验证：

```bash
brew style xuhuanxxx/tap/open-edifier
brew audit --strict --online xuhuanxxx/tap/open-edifier
brew install --build-from-source xuhuanxxx/tap/open-edifier
brew test xuhuanxxx/tap/open-edifier
edifier --version
edifier --help
brew uninstall open-edifier
```

alpha 阶段保持自有 tap。源码安装耗时真正成为问题后再增加 bottles；满足稳定性和外部使用要求后再评估 `homebrew/core`。

Homebrew 参考：[Tap 指南](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)、[Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)和[`std_cargo_args`](https://docs.brew.sh/rubydoc/Formula.html#std_cargo_args-instance_method)。

公开前已经用当前干净 commit 的源码归档和临时本地 tap 验证上述 `install` 与 `test` 逻辑，Homebrew 能识别 `0.1.0-alpha.1`、完成源码构建、执行版本测试并卸载。该预验不包含尚不存在的 public tag URL、线上 SHA-256、`audit --online` 或升级路径，不能替代正式 tap 验收。

### macOS App

首个公开 alpha 沿用现有 ad-hoc 签名，不要求 Apple Developer Program、Developer ID 或公证。发布产物包括：

```text
OpenEdifier-<version>-macos-arm64.dmg
OpenEdifier-<version>-macos-arm64.dmg.sha256
```

构建、签名、打包和 checksum 验证由一个脚本完成：

```bash
apps/macos/package.sh
```

图标由仓库内的 Swift 绘制源码确定性生成，bundle 版本来自 workspace，DMG 同时包含 `/Applications` 快捷方式。具体产物和发布命令见[发布操作说明](release.md)。

由于没有 Developer ID，Release notes 必须说明产物未经过 Apple 身份认证和公证。用户首次尝试打开后，只能在确认仓库来源和 checksum 的前提下，通过“系统设置 → 隐私与安全性 → 仍要打开”授权单个 App。不得要求用户全局关闭 Gatekeeper，也不得把删除 quarantine 属性作为默认安装方式。参见 Apple 的[未知开发者 App 打开说明](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac)。

只有项目已有 Apple Developer Program 账号，或真实反馈证明手动授权严重阻碍使用时，才评估 Developer ID 和公证。

## P0：可追踪发布

- 首个公开 tag 使用 `v0.1.0-alpha.1`；
- workspace version、`edifier --version`、App version、DMG 文件名和 Release notes 必须对应；
- 所有产物从通过候选 CI 和 Homebrew 验收的 tag 构建，不从未提交工作区上传；
- alpha 和 beta 使用 GitHub prerelease 标记；
- tag 推送只验证候选，不直接发布；Homebrew 线上验收通过后，人工触发同一 workflow 创建 prerelease；
- GitHub Release 附带 Homebrew 安装命令、DMG、checksum、支持边界、已知问题和实际测试结果；
- CI 未执行的 Homebrew、安装、公证或实机场景不得描述为已经通过。

参考 [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)。实际命令和人工门见[发布操作说明](release.md)。

## P0：项目可维护和可扩展

多型号最小重构已经落地：core 不再包含 S260 常量；`DeviceStatus` 使用可选状态和稳定的 `DeviceCapabilities`；不支持 source、volume、EQ 或 playback 的驱动可以使用默认能力错误；facade 通过一个静态表集中注册驱动；CLI、C ABI 和 macOS App 不再写型号分支，macOS 控件根据驱动能力显示。

这证明新驱动可以接入现有调度和产品入口，但不能证明某个未验证硬件兼容。当前 discovery 仍只实现经过 S260 确认的 AirPlay mDNS 路径；第二型号若使用其他发现方式，再根据真实广播证据增加 discoverer。

完整扩展点、测试证据和新型号改动范围见[架构设计](architecture.md#新型号扩展准备度)。第二个型号出现前，不再增加插件系统、通用传输框架、宏注册表或 async runtime。

## 发布候选验收

### 自动质量门

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo package --workspace --allow-dirty --no-verify
apps/macos/package.sh
```

### S260 实机验收

测试前记录输入源、音量、EQ 和播放状态。验证发现、状态同步、小幅音量修改及恢复、EQ 修改及恢复、可用输入源切换、播放 ACK、设备离线错误和重新发现。

不执行恢复出厂、固件升级、关机、重命名或网络破坏性测试。发布记录必须说明已验证项、未验证项和设备最终状态。

### 安装验收

- 从全新 clone 执行 README 构建步骤成功；
- Homebrew 可安装、测试、升级和卸载 CLI；
- macOS App 在没有 Rust、CLI 或开发环境的用户环境启动；
- 未认证开发者提示和公开说明一致，单 App 授权后可运行；
- App 不依赖 daemon 或 launch agent；
- CLI 和 App 都只暴露经过验证的 S260 能力。

## 完成定义

满足以下条件，才表示 OpenEdifier 开源项目完成首个公开 alpha 闭环：

- 公开前隐私和专有产物审计通过；
- GitHub public 仓库具备最小公开文件和 CI；
- README 能让陌生用户理解边界并从源码运行项目；
- commit、tag、version、changelog 和发布产物可追踪；
- Homebrew CLI 安装、测试和卸载通过；
- macOS DMG、checksum、风险说明和安装验证齐全；
- Rust、SwiftUI、文档和 S260 实机质量门通过，设备状态已恢复；
- 新型号扩展边界有文档，但没有把未验证型号描述成支持；
- iOS、公开绑定和 Home Assistant 明确标记为路线图。

## 延后事项

| 事项 | 启动条件 |
|---|---|
| Issue / PR 模板和更严格仓库规则 | 出现外部贡献或维护噪音 |
| 自动更新 | 已有稳定发布节奏和真实用户群 |
| Homebrew bottles | 源码安装耗时成为真实问题 |
| `homebrew/core` | 已有稳定版本、平台覆盖和外部使用 |
| Developer ID 与 Apple 公证 | 已有开发者账号，或手动授权严重阻碍使用 |
| Universal Binary | 确认存在 Intel Mac 用户和验证环境 |
| 手动 IP UI | 出现可复现的 mDNS 失败 |
| macOS 实时事件 | 轮询造成可观察的状态错误 |
| 新型号 | 具备型号、固件、脱敏 fixture、mock 或实机证据 |
| iOS、公开绑定、Home Assistant | 有明确使用需求并进入实际开发 |
| daemon、账号、云服务、遥测 | 当前不规划 |
