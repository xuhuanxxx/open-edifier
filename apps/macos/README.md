# OpenEdifier macOS 应用

可运行的原生 SwiftUI MVP，直接通过静态链接的 Rust bridge 使用 OpenEdifier SDK，不依赖 CLI、后台服务或网络 API。

## 当前功能

- 自动发现局域网中的受支持音箱；
- 查看型号、固件、输入源、音量、EQ 和播放状态；
- 切换蓝牙、AUX、USB 和 AirPlay；
- 调节音量和 EQ；
- 播放、暂停、上一首和下一首；
- 修改后读取音箱状态进行确认；
- 记住上次选择的音箱，并每 5 秒静默同步状态；
- 支持 `⌘R` 刷新、控制菜单和 `⌘+`、`⌘-` 调节音量；
- 输入源、音量、EQ 和播放控件根据驱动公开能力动态显示；
- 使用 SwiftUI `WindowGroup` 接入标准 macOS 应用菜单和窗口管理；
- 设备离线和协议错误提示。

## 本机构建

当前构建脚本面向 Apple Silicon 和 macOS 26：

```bash
apps/macos/build.sh
open apps/macos/dist/OpenEdifier.app
```

产物位于 `apps/macos/dist/OpenEdifier.app`，已经进行 ad-hoc 签名。首次启动时，macOS 可能请求本地网络访问权限。

生成带应用图标、`/Applications` 快捷方式和 SHA-256 的 DMG：

```bash
apps/macos/package.sh
```

产物位于 `apps/macos/dist/`，文件名包含完整项目版本和 `arm64` 架构。图标由仓库中的 `GenerateAppIcon.swift` 确定性生成，不依赖外部设计文件。

当前 App 没有 Developer ID，也没有经过 Apple 公证。确认仓库来源和 checksum 后，首次尝试打开，再前往“系统设置 → 隐私与安全性”选择“仍要打开”。不要全局关闭 Gatekeeper。

公开 alpha 前仍需完成非开发用户环境安装验证。Developer ID、公证、Universal Binary 和更低 macOS 部署目标只由真实需求触发，不作为首发阻塞项。
