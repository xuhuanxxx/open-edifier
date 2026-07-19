#!/bin/zsh
set -euo pipefail

script_dir=${0:A:h}
repo_root=${script_dir:h:h}
app_name=OpenEdifier
app_bundle="$script_dir/dist/$app_name.app"
app_contents="$app_bundle/Contents"
target_dir="$repo_root/target/release"

cd "$repo_root"
MACOSX_DEPLOYMENT_TARGET=26.0 cargo build --locked --release -p open-edifier-swift-bridge

mkdir -p "$app_contents/MacOS" "$app_contents/Resources"
cp "$script_dir/Info.plist" "$app_contents/Info.plist"

xcrun swiftc \
  -swift-version 5 \
  -warnings-as-errors \
  -parse-as-library \
  -target arm64-apple-macosx26.0 \
  -import-objc-header "$script_dir/OpenEdifierBridge.h" \
  "$script_dir/OpenEdifierSwiftUI.swift" \
  "$target_dir/libopen_edifier_swift_bridge.a" \
  -framework AppKit \
  -framework Foundation \
  -framework SwiftUI \
  -o "$app_contents/MacOS/$app_name"

codesign --force --sign - --timestamp=none "$app_bundle"
echo "$app_bundle"
