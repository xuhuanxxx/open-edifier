#!/bin/zsh
set -euo pipefail

script_dir=${0:A:h}
repo_root=${script_dir:h:h}
app_name=OpenEdifier
app_bundle="$script_dir/dist/$app_name.app"
app_contents="$app_bundle/Contents"
target_dir="$repo_root/target/release"
icon_work_dir=$(mktemp -d "${TMPDIR:-/tmp}/open-edifier-icon.XXXXXX")
trap 'rm -rf "$icon_work_dir"' EXIT
release_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml")
short_version=${release_version%%-*}
build_number=${OPEN_EDIFIER_BUILD_NUMBER:-1}

if [[ -z "$release_version" || "$release_version" == *$'\n'* ]]; then
  print -u2 "无法从 workspace Cargo.toml 读取唯一版本"
  exit 1
fi
case "$build_number" in
  ''|*[!0-9]*)
    print -u2 "OPEN_EDIFIER_BUILD_NUMBER 必须是非负整数"
    exit 1
    ;;
esac

cd "$repo_root"
MACOSX_DEPLOYMENT_TARGET=26.0 cargo build --locked --release -p open-edifier-swift-bridge

rm -rf "$app_bundle"
mkdir -p "$app_contents/MacOS" "$app_contents/Resources"
cp "$script_dir/Info.plist" "$app_contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$short_version" "$app_contents/Info.plist"
plutil -replace CFBundleVersion -string "$build_number" "$app_contents/Info.plist"
plutil -insert OpenEdifierReleaseVersion -string "$release_version" "$app_contents/Info.plist"

iconset_dir="$icon_work_dir/OpenEdifier.iconset"
base_icon="$icon_work_dir/OpenEdifier-1024.png"
mkdir -p "$iconset_dir"
xcrun swift "$script_dir/GenerateAppIcon.swift" "$base_icon"
sips -z 16 16 "$base_icon" --out "$iconset_dir/icon_16x16.png" >/dev/null
sips -z 32 32 "$base_icon" --out "$iconset_dir/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$base_icon" --out "$iconset_dir/icon_32x32.png" >/dev/null
sips -z 64 64 "$base_icon" --out "$iconset_dir/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$base_icon" --out "$iconset_dir/icon_128x128.png" >/dev/null
sips -z 256 256 "$base_icon" --out "$iconset_dir/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$base_icon" --out "$iconset_dir/icon_256x256.png" >/dev/null
sips -z 512 512 "$base_icon" --out "$iconset_dir/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$base_icon" --out "$iconset_dir/icon_512x512.png" >/dev/null
cp "$base_icon" "$iconset_dir/icon_512x512@2x.png"
iconutil -c icns "$iconset_dir" -o "$app_contents/Resources/OpenEdifier.icns"

xcrun swiftc \
  -swift-version 5 \
  -warnings-as-errors \
  -parse-as-library \
  -target arm64-apple-macosx26.0 \
  -import-objc-header "$script_dir/OpenEdifierBridge.h" \
  "$script_dir/StorePolicy.swift" \
  "$script_dir/OpenEdifierSwiftUI.swift" \
  "$target_dir/libopen_edifier_swift_bridge.a" \
  -framework AppKit \
  -framework Foundation \
  -framework SwiftUI \
  -o "$app_contents/MacOS/$app_name"

codesign --force --sign - --timestamp=none "$app_bundle"
echo "$app_bundle"
