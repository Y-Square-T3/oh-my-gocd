#!/usr/bin/env bash
set -euo pipefail

test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT

dist_dir="$test_root/dist"
mkdir -p "$dist_dir"
printf linux > "$dist_dir/omg-linux-amd64.tar.gz"
printf macos > "$dist_dir/omg-macos-arm64.tar.gz"
printf windows > "$dist_dir/omg-windows-amd64.zip"

sh "$PWD/scripts/prepare-release-assets.sh" "$dist_dir"

test -f "$dist_dir/install.sh"
test -f "$dist_dir/install.ps1"
if grep -F "$dist_dir/" "$dist_dir/SHA256SUMS" >/dev/null; then
  echo "SHA256SUMS contains distribution-directory paths" >&2
  exit 1
fi
(
  cd "$dist_dir"
  sha256sum --check SHA256SUMS
)

echo "Release asset tests passed"
