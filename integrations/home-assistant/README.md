# Home Assistant 集成

规划中的本地集成：使用 `bindings/python` 提供的纯 Python 异步客户端和二进制状态事件，不调用 CLI 或 Rust 产物。设备发现使用 Home Assistant 自带 Zeroconf；重连后或事件无法提供完整状态快照时，显式刷新状态。
