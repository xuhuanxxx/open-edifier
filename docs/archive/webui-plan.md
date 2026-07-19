# WebUI 纯前端方案（已归档）

> 文档类型：过程方案
> 当前状态：已归档，不在产品路线图中
> 产品约束：公共静态部署、纯前端、无云后端、无本地服务

本方案保留为项目研究记录。结论是：公共静态站点可以部署，但普通浏览器无法连接 S260 已验证的原始 TCP/mDNS 控制面；加入本地 companion 又违背纯前端目标，因此项目停止 WebUI 产品开发。

## 目标

WebUI 希望成为一个通过公共 URL 访问的音箱控制台。站点本身只包含 HTML、CSS、JavaScript 和可选的 WebAssembly，可以部署到 GitHub Pages、Cloudflare Pages 或同类静态托管服务。

这个目标包含两个不同问题：

1. 能否把界面部署为公共纯前端应用：可以。
2. 普通浏览器能否从该页面直接控制当前 S260：以已验证协议看，目前不可以。

第二点是 MVP 的前置门禁。没有传输能力，先完成漂亮界面并不构成可用产品。

## 已知约束

S260 的控制路径是局域网 TCP `8080` 上的私有 framed JSON 与 AAEC 二进制协议，并通过 mDNS 发现设备。它不是 HTTP 或 WebSocket 服务。

普通公共网页只能使用浏览器开放的网络 API，不能建立任意原始 TCP/UDP 连接。将 Rust 编译为 `wasm32-unknown-unknown` 不会增加权限：

- 可以 Wasm 化：数据类型、校验、帧编解码和纯状态转换；
- 不能由浏览器 Wasm 直接完成：`TcpStream`、mDNS 和当前设备事件 socket；
- WebSocket 不能连接原始 TCP 服务，因为连接握手和帧格式不同；
- PWA 和 Service Worker 不会获得原始 socket 权限；
- WebUSB/WebHID 不能替代尚未发现的音箱设置通道；当前 USB 研究只确认了 Audio 与 Consumer Control 行为；
- Direct Sockets 面向受信任的 Isolated Web App，不是普通公共 URL 的通用能力，也不满足当前跨浏览器目标。

## 产品红线

- 不增加云端代理。云服务通常也无法访问用户家庭局域网内的音箱。
- 不把 localhost helper、浏览器扩展或原生壳暗称为“纯前端”。
- 不使用 DNS rebinding、关闭浏览器安全策略或其他不安全绕行方案。
- 不在传输未打通时把 mock UI 宣称为 MVP。
- 不让 WebUI 的临时方案成为 CLI、iOS、macOS 或 Home Assistant 的公共控制面。

## 第一阶段：传输可行性门禁

在建立 UI 工程前完成一份可复现的浏览器实验报告：

1. 重新枚举 S260 暴露的局域网服务，确认是否存在尚未记录的 HTTP、HTTPS、WebSocket 或 WebTransport 端点；
2. 对候选端点只做最小、非破坏性握手，禁止端口扫描扩大到无关设备；
3. 在 Chromium、Safari 和 Firefox 中确认公共 HTTPS 页面可用的局域网权限与限制；
4. 验证 Direct Sockets 是否只在 Isolated Web App 环境可用；
5. 记录 WebUSB/WebHID 描述符是否存在设备设置输出报告，不从 Consumer Control 接口臆测控制能力；
6. 给出明确的 `可行` 或 `不可行` 结论和原始证据。

门禁通过的唯一条件是：普通公共网页可以在用户明确授权后，通过已验证且安全的浏览器 API 直接完成至少一次状态读取和一次可恢复的输入源切换。

## 门禁后的路线

### A. 找到浏览器兼容端点

如果音箱确实存在 HTTP、WebSocket 或其他标准浏览器端点：

- 在 Rust 纯协议 crate 中整理可 Wasm 化部分；
- 用 `wasm-bindgen` 暴露最小编解码接口；
- 浏览器使用原生 API 承担传输；
- 真机验证状态、输入源、音量、EQ、播放和事件能力；
- 再进入 UI MVP。

这是唯一完全满足“公共纯前端实控”的路线。

### B. 没有浏览器兼容端点

如果门禁失败，应明确停止纯 Web 控制台实现，并从以下产品方向另选一个，不能假装限制不存在：

- 公共协议 Playground：纯前端，只做文档、fixture 解码和交互演示，不宣称控制真机；
- 可选本地 companion：公共前端连接用户主动安装的本地程序，但产品不再是零本地后端；
- Tauri/macOS/iOS 原生产品：复用 Web UI 或设计系统，通过原生 Rust/Swift 传输控制设备；
- Isolated Web App 实验：只作为受限平台研究，不作为跨平台公共 Web 产品。

## UI 技术栈草案

只有传输门禁通过，或明确选择“公共协议 Playground”后，才创建 UI 工程。

推荐保持小而透明：

- Preact；
- 严格 TypeScript；
- Vite；
- 原生 `fetch`、WebSocket 或对应的已验证浏览器 API；
- CSS 设计令牌和组件级 CSS；
- Biome；
- Vitest、Testing Library、MSW 和少量 Playwright 用例。

暂不引入 Router、TanStack Query、Zustand、Tailwind、通用组件库和服务端框架。当前是单页设备控制器，不需要这些额外抽象。

## UI MVP 范围

传输门禁通过后的第一个真实 MVP 只包含：

- 用户授权并选择设备；
- 读取设备、固件、输入源、音量和 EQ；
- 切换输入源；
- 调节音量和 EQ；
- 播放、暂停、上一首和下一首；
- 显示实时事件或明确的刷新状态；
- 完整的加载、错误、权限拒绝和设备离线反馈；
- 键盘操作与移动端布局。

所有修改操作都必须读取设备状态进行确认，不能只根据前端本地状态显示成功。

## 归档结论

不再创建 UI 工程，也不再安排浏览器传输门禁。只有未来音箱出现经过验证的 HTTP、WebSocket 或其他标准浏览器控制端点，或者主流浏览器为公共站点提供安全、跨平台的原始局域网 socket 能力时，才重新评估本方向。
