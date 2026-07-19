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
- 支持 `⌘R` 刷新、`⌘1` 至 `⌘4` 切换输入源、`⌘+` 与 `⌘-` 调节音量；
- 使用 SwiftUI `WindowGroup` 接入标准 macOS 应用菜单和窗口管理；
- 设备离线和协议错误提示。

## 本机构建

当前构建脚本面向 Apple Silicon 和 macOS 26：

```bash
apps/macos/build.sh
open apps/macos/dist/OpenEdifier.app
```

产物位于 `apps/macos/dist/OpenEdifier.app`，已经进行 ad-hoc 签名。首次启动时，macOS 可能请求本地网络访问权限。

公开 alpha 前仍需补充正式图标、DMG 打包、checksum、未认证应用安装说明和干净环境验证。首发不要求付费 Apple Developer Program；Developer ID 和公证只作为未来可选升级。Universal Binary 和更低 macOS 部署目标同样由真实用户需求触发。
