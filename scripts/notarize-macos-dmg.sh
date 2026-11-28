#!/usr/bin/env bash
set -euo pipefail

target=$1
assets=$2
case "$target" in
  aarch64-apple-darwin) arch=arm64 ;;
  x86_64-apple-darwin) arch=x86_64 ;;
  *) exit 1 ;;
esac
shopt -s nullglob
dmgs=("$assets"/*.dmg)
archives=("$assets"/*.app.tar.gz)
[[ ${#dmgs[@]} == 1 && ${#archives[@]} == 1 ]]
dmg=${dmgs[0]}
archive=${archives[0]}
work=$(mktemp -d)
mount="$work/mount"
mkdir "$mount" "$work/updater"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then hdiutil detach "$mount" || exit 1; fi
  rm -rf "$work"
}
trap cleanup EXIT

verify_app() {
  local app=$1
  [[ "$(lipo -archs "$app/Contents/MacOS/jajan")" == "$arch" ]]
  codesign --verify --deep --strict "$app"
  xcrun stapler validate "$app"
}

tar -xzf "$archive" -C "$work/updater"
verify_app "$work/updater/Jajan.app"
xcrun notarytool submit "$dmg" --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" \
  --issuer "$APPLE_API_ISSUER" --wait --timeout 45m --output-format json > "$work/submission.json"
id=$(jq -er '.id | select(test("^[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$"))' "$work/submission.json")
status=$(jq -er '.status' "$work/submission.json")
echo "$target DMG submission $id ($status)"
[[ "$status" == Accepted ]]
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"
hdiutil attach -readonly -nobrowse -mountpoint "$mount" "$dmg"
mounted=true
verify_app "$mount/Jajan.app"
{
  echo "### $target"
  echo "DMG submission \`$id\` — $status"
  echo
  echo 'stapler validate passed on DMG, mounted app and updater app.'
  echo '~~~'
  shasum -a 256 "$dmg" "$archive"
  echo '~~~'
} >> "$GITHUB_STEP_SUMMARY"
