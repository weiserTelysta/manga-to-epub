#!/usr/bin/env bash
set -euo pipefail
telysta_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$telysta_root"
if [[ "$(uname -s)" != "Darwin" ]]; then
  printf 'Run this script on macOS with the Xcode command line tools installed.\n' >&2
  exit 1
fi
export MACOSX_DEPLOYMENT_TARGET=11.0
if [[ "${1:-}" != "--skip-checks" ]]; then
  cargo fmt --check
  cargo clippy --locked --all-targets -- -D warnings
  cargo test --locked
fi
rustup target add x86_64-apple-darwin aarch64-apple-darwin
for telysta_target in x86_64-apple-darwin aarch64-apple-darwin; do
  cargo build --release --locked --bins --target "$telysta_target"
done
telysta_build="${CARGO_TARGET_DIR:-target}"
telysta_package="dist/Telysta-macOS-universal"
telysta_app="$telysta_package/Telysta.app"
mkdir -p "$telysta_app/Contents/MacOS" "$telysta_app/Contents/Resources" "$telysta_package/docs"
for telysta_binary in telysta telysta-cli; do
  if [[ "$telysta_binary" == "telysta" ]]; then
    telysta_destination="$telysta_app/Contents/MacOS/Telysta"
  else
    telysta_destination="$telysta_package/telysta-cli"
  fi
  lipo -create "$telysta_build/x86_64-apple-darwin/release/$telysta_binary" "$telysta_build/aarch64-apple-darwin/release/$telysta_binary" -output "$telysta_destination"
  chmod 755 "$telysta_destination"
  lipo "$telysta_destination" -verify_arch x86_64 arm64
done
cp assets/macos/Info.plist "$telysta_app/Contents/Info.plist"
cp assets/telysta.icns "$telysta_app/Contents/Resources/telysta.icns"
codesign --force --deep --sign - "$telysta_app"
codesign --verify --deep --strict "$telysta_app"
cp README.md LICENSE THIRD-PARTY-NOTICES.md "$telysta_package/"
cp assets/fonts/OFL.txt "$telysta_package/FONT-LICENSE.txt"
cp assets/licenses/UNRAR.txt "$telysta_package/UNRAR-LICENSE.txt"
cp docs/使用说明.txt "$telysta_package/使用说明.txt"
cp docs/preview.png docs/UI-DESIGN.md docs/PLATFORMS.md docs/DEVELOPMENT.md docs/MANGA-WORKFLOW.md docs/READER-COMPATIBILITY.md docs/VALIDATION.md "$telysta_package/docs/"
ditto -c -k --sequesterRsrc --keepParent "$telysta_package" dist/Telysta-macOS-universal.zip
printf 'Ready: %s\n' "$telysta_app" dist/Telysta-macOS-universal.zip
