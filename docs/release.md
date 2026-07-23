# 发布操作说明

OpenEdifier 的 release 必须对应一个已经通过质量门的干净 commit。主仓库公开后，先用 tag 固定发布候选并完成 Homebrew 线上验证，再人工触发 GitHub prerelease；不能让 tag 推送直接发布未经安装验收的产物。

## 版本来源

workspace `Cargo.toml` 中的 `version` 是项目 release 版本来源。当前候选版本为 `0.2.0-alpha.1`，计划对应 tag `v0.2.0-alpha.1`。

- `edifier --version` 输出完整版本；
- macOS `CFBundleShortVersionString` 使用 `0.2.0`；
- bundle 中的 `OpenEdifierReleaseVersion` 保留完整预发布版本；
- DMG 文件名包含完整版本和架构。
- Python 包使用对应的 PEP 440 版本 `0.2.0a1`；发布到 PyPI 前仍以源码 wheel 验证，不上传产物。

## 发布候选检查

在创建 tag 前执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo package --workspace --allow-dirty --no-verify
PYTHONPATH=bindings/python/src python3 -m unittest discover -s bindings/python/tests
python3 -m pip wheel --no-deps --wheel-dir /tmp/open-edifier-wheel bindings/python
apps/macos/package.sh
cargo run --locked --quiet -p open-edifier-cli -- --version
```

还必须：

1. 从全新 clone 重复 README 的源码安装步骤；
2. tag 推送后的候选 workflow 通过，再完成 Homebrew formula 的 style、audit、install、test 和 uninstall；
3. 按实机规则验证 S260 并恢复原状态；
4. 更新 `CHANGELOG.md` 和对应版本的 release notes，并在 tag 前将版本标题中的“尚未发布”替换为实际发布日期；
5. 确认 release notes 中的实机记录对应当前发布候选，并包含设备最终状态；
6. 确认 `main` 与 `origin/main` 一致且工作区干净。

## macOS 产物

`apps/macos/package.sh` 会确定性生成：

```text
apps/macos/dist/OpenEdifier-<version>-macos-arm64.dmg
apps/macos/dist/OpenEdifier-<version>-macos-arm64.dmg.sha256
```

脚本会重新构建 App、生成 `.icns`、注入版本、执行 ad-hoc 签名、创建带 `/Applications` 快捷方式的 DMG，并回读 SHA-256。

发布前还要在非开发用户环境挂载 DMG、拖入 `/Applications`，并验证 Apple 提供的单 App“仍要打开”流程。未认证风险必须保留在 release notes 中。`v0.1.0-alpha.1` 已完成该人工验收。

## Homebrew tap

主仓库转为 public 且 release tag 可下载后，创建 `xuhuanxxx/homebrew-tap` public 仓库和 `Formula/open-edifier.rb`。formula 固定到 tag tarball 和 SHA-256，只安装 `edifier` CLI。

当前已经使用干净 commit 的本地源码归档和临时 tap 验证 formula 的 style、源码安装、test、version/help 和卸载。这个结果只证明构建与测试逻辑有效；正式发布仍必须使用 public tag tarball 重新计算 SHA-256，并完成 online audit、升级和卸载验证。

```bash
brew style xuhuanxxx/tap/open-edifier
brew audit --strict --online xuhuanxxx/tap/open-edifier
brew install --build-from-source xuhuanxxx/tap/open-edifier
brew test xuhuanxxx/tap/open-edifier
edifier --version
edifier --help
brew uninstall open-edifier
```

正式 formula 验证需要公开 tag tarball，因此 tag 是候选源码锚点。Homebrew 验证没有完成前，不人工触发 GitHub prerelease，避免 Release 已发布但 README 中的安装命令不可用。

## 创建 release

所有人工检查完成并提交后：

```bash
git tag -a v0.2.0-alpha.1 -m "OpenEdifier 0.2.0-alpha.1"
git push origin v0.2.0-alpha.1
```

tag 会触发 `.github/workflows/release.yml`，再次验证版本、release notes、Rust 质量门和 macOS 打包，但不会立刻创建 Release。候选 workflow 和 Homebrew 线上验收都通过后，再执行：

```bash
gh workflow run release.yml --ref v0.2.0-alpha.1 -f tag=v0.2.0-alpha.1
```

人工触发的 workflow 会重新从该 tag 构建，并创建 GitHub prerelease。不要从未提交的本地 `dist/` 手工上传产物。

如果候选失败，修复后使用新的预发布版本；不要移动已经公开使用的 tag，也不要覆盖既有 release 产物。
