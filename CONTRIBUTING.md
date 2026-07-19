# 贡献指南

欢迎为更多 EDIFIER 型号贡献支持。每个协议族应放在独立的 Rust crate 中，并提供经过脱敏的捕获 fixture 或确定性的模拟测试。禁止提交厂商 APK、固件、凭据、MAC 地址或私有网络信息。

提交 Pull Request 前请运行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo package --workspace --allow-dirty --no-verify
```

内部 crate 尚未发布到 crates.io 时需要使用 `--no-verify`。发布顺序依次为 core、AAEC framing、S260 driver、discovery、facade、CLI；当前置依赖已经进入 registry 后，应恢复正常的 package 验证。

型号驱动应实现共享的 `Device` 契约，在修改后验证设备状态。恢复出厂、固件升级等破坏性能力必须具备书面安全设计，否则不得进入公开 API。发现未知型号并不意味着能够控制它；不要猜测其他型号与 S260 共用端口或 wire format。
