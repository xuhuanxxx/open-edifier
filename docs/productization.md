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

已经具备：

- 型号无关的同步 Rust 公共契约；
- S260 framed JSON 控制和 `BB EC` 实时事件；
- 输入源、音量和 EQ 写后验证；
- mDNS 发现、CLI 和机器可读 JSON 输出；
- 通过最小 C ABI 静态链接 Rust 能力的 SwiftUI macOS MVP；
- mock、协议文档、架构文档和 Rust workspace 质量门；
- 新型号所需的最小驱动接缝。

公开前仍需完成：

- 隐私、凭据、真实设备信息和专有产物审计；
- 可追踪的初始 commit、tag 和 changelog；
- GitHub public 所需的 README、许可证、贡献和安全说明；
- Rust 和 macOS 构建 CI；
- CLI `--version` 和 Homebrew tap；
- macOS 图标、DMG、checksum 和未认证应用安装说明；
- 从干净 tag 构建发布产物的可重复流程；
- Homebrew、macOS 安装和 S260 实机验收。

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

当前仓库尚无 commit，应在创建公开历史前检查所有文件：

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
2. macOS runner 执行 `apps/macos/build.sh` 并验证 ad-hoc 签名。

普通 CI 不需要证书、Apple 账号、设备地址或其他私密环境。

## P0：项目可安装和可使用

### Homebrew CLI

创建独立 public tap：

```text
<github-owner>/homebrew-tap
  Formula/
    open-edifier.rb
```

用户安装：

```bash
brew install <github-owner>/tap/open-edifier
```

formula 名称使用 `open-edifier`，安装二进制保持 `edifier`。它只构建 CLI，不构建 macOS App。

```ruby
class OpenEdifier < Formula
  desc "Local-first CLI for supported EDIFIER speakers"
  homepage "https://github.com/<github-owner>/open-edifier"
  url "https://github.com/<github-owner>/open-edifier/archive/refs/tags/v<version>.tar.gz"
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
brew style <github-owner>/tap/open-edifier
brew audit --strict --online <github-owner>/tap/open-edifier
brew install --build-from-source <github-owner>/tap/open-edifier
brew test <github-owner>/tap/open-edifier
edifier --version
edifier --help
brew uninstall open-edifier
```

alpha 阶段保持自有 tap。源码安装耗时真正成为问题后再增加 bottles；满足稳定性和外部使用要求后再评估 `homebrew/core`。

Homebrew 参考：[Tap 指南](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)、[Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)和[`std_cargo_args`](https://docs.brew.sh/rubydoc/Formula.html#std_cargo_args-instance_method)。

### macOS App

首个公开 alpha 沿用现有 ad-hoc 签名，不要求 Apple Developer Program、Developer ID 或公证。发布产物包括：

```text
OpenEdifier-<version>-macos-arm64.dmg
OpenEdifier-<version>-macos-arm64.dmg.sha256
```

构建和验证：

```bash
apps/macos/build.sh
codesign --verify --deep --strict --verbose=2 apps/macos/dist/OpenEdifier.app
hdiutil create -volname OpenEdifier \
  -srcfolder apps/macos/dist/OpenEdifier.app \
  -format UDZO OpenEdifier.dmg
shasum -a 256 OpenEdifier.dmg
```

由于没有 Developer ID，Release notes 必须说明产物未经过 Apple 身份认证和公证。用户首次尝试打开后，只能在确认仓库来源和 checksum 的前提下，通过“系统设置 → 隐私与安全性 → 仍要打开”授权单个 App。不得要求用户全局关闭 Gatekeeper，也不得把删除 quarantine 属性作为默认安装方式。参见 Apple 的[未知开发者 App 打开说明](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac)。

只有项目已有 Apple Developer Program 账号，或真实反馈证明手动授权严重阻碍使用时，才评估 Developer ID 和公证。

## P0：可追踪发布

- 首个公开 tag 使用 `v0.1.0-alpha.1`；
- workspace version、`edifier --version`、App version、DMG 文件名和 Release notes 必须对应；
- 所有产物从通过 CI 的 tag 构建，不从未提交工作区上传；
- alpha 和 beta 使用 GitHub prerelease 标记；
- GitHub Release 附带 Homebrew 安装命令、DMG、checksum、支持边界、已知问题和实际测试结果；
- CI 未执行的 Homebrew、安装、公证或实机场景不得描述为已经通过。

参考 [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)。

## P0：项目可维护和可扩展

当前多型号架构已经提供第二型号需要的最小接缝：字符串形式的 `ModelId` 和 `Source`、同步 `Device` 契约、可选 EQ 和播放状态、未知事件保留、未知发现型号保留、型号独立驱动 crate，以及由 facade 选择驱动和默认端口。

这表示项目可以在不重写 S260 驱动、CLI 或绑定协议的前提下加入第二个真实型号，但不表示任意型号已经即插即用。当前仍有四个必须由真实型号验证的假设：

1. `open-edifier-core` 中仍存在 `MODEL_S260` 常量；
2. facade 在多个入口手工匹配驱动；
3. `DeviceStatus` 和 `Device` 默认假设设备具有输入源和音量能力；
4. discovery 当前只实现已验证的 AirPlay mDNS 路径。

macOS UI 还写死了 S260 的四个输入源和播放控件。新增型号时，应用必须根据公开状态和能力渲染，不能复制型号判断或 wire protocol。

这些限制不应现在通过插件系统、通用传输框架或大规模 trait 设计“解决”。第二个型号出现后，按证据完成新驱动、发现映射、facade 注册和应用能力适配；只有真实差异无法落入现有契约时才修改 core。完整边界见[架构设计](architecture.md#新型号扩展准备度)。

## 发布候选验收

### 自动质量门

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo package --workspace --allow-dirty --no-verify
apps/macos/build.sh
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
