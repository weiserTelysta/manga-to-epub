#!/usr/bin/env bash
set -euo pipefail
telysta_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$telysta_root"
if [[ "${1:-}" != "--skip-checks" ]]; then
  cargo fmt --check
  cargo clippy --locked --all-targets -- -D warnings
  cargo test --locked
fi
cargo build --release --locked --bins
telysta_arch="$(uname -m)"
telysta_target="${CARGO_TARGET_DIR:-target}"
telysta_package="dist/Telysta-Linux-$telysta_arch"
mkdir -p "$telysta_package/docs"
install -m 755 "$telysta_target/release/telysta" "$telysta_package/telysta"
install -m 755 "$telysta_target/release/telysta-cli" "$telysta_package/telysta-cli"
cp README.md LICENSE THIRD-PARTY-NOTICES.md "$telysta_package/"
cp assets/fonts/OFL.txt "$telysta_package/FONT-LICENSE.txt"
cp assets/licenses/UNRAR.txt "$telysta_package/UNRAR-LICENSE.txt"
cp assets/telysta.png "$telysta_package/telysta.png"
cp assets/linux/telysta.desktop "$telysta_package/Telysta.desktop"
cp docs/使用说明.txt "$telysta_package/使用说明.txt"
cp docs/preview.png docs/UI-DESIGN.md docs/PLATFORMS.md docs/DEVELOPMENT.md docs/MANGA-WORKFLOW.md docs/READER-COMPATIBILITY.md docs/VALIDATION.md "$telysta_package/docs/"
tar -czf "dist/Telysta-Linux-$telysta_arch.tar.gz" -C dist "Telysta-Linux-$telysta_arch"
printf 'Ready: %s\n' "$telysta_package/telysta" "dist/Telysta-Linux-$telysta_arch.tar.gz"
