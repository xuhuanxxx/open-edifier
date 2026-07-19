#!/bin/zsh
set -euo pipefail

script_dir=${0:A:h}
repo_root=${script_dir:h:h}
dist_dir="$script_dir/dist"
app_name=OpenEdifier
app_bundle="$dist_dir/$app_name.app"
release_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml")

if [[ -z "$release_version" || "$release_version" == *$'\n'* ]]; then
  print -u2 "无法从 workspace Cargo.toml 读取唯一版本"
  exit 1
fi

dmg_name="$app_name-$release_version-macos-arm64.dmg"
dmg_path="$dist_dir/$dmg_name"
checksum_path="$dmg_path.sha256"
staging_dir=$(mktemp -d "${TMPDIR:-/tmp}/open-edifier-dmg.XXXXXX")
trap 'rm -rf "$staging_dir"' EXIT

"$script_dir/build.sh"
codesign --verify --deep --strict --verbose=2 "$app_bundle"

ditto "$app_bundle" "$staging_dir/$app_name.app"
ln -s /Applications "$staging_dir/Applications"
rm -f "$dmg_path" "$checksum_path"
hdiutil create \
  -volname "$app_name" \
  -srcfolder "$staging_dir" \
  -format UDZO \
  "$dmg_path"

(
  cd "$dist_dir"
  shasum -a 256 "$dmg_name" > "$dmg_name.sha256"
  shasum -a 256 -c "$dmg_name.sha256"
)

echo "$dmg_path"
echo "$checksum_path"
