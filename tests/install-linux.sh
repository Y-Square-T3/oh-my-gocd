#!/usr/bin/env bash
set -euo pipefail

test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT

release_dir="$test_root/release"
package_dir="$test_root/package"
install_dir="$test_root/install"
fake_bin="$test_root/fake-bin"
mkdir -p "$release_dir" "$package_dir" "$fake_bin"

cat > "$fake_bin/cargo" <<'EOF'
#!/bin/sh
echo "installer invoked cargo" >&2
exit 99
EOF
chmod +x "$fake_bin/cargo"

cat > "$package_dir/omg" <<'EOF'
#!/bin/sh
printf '%s\n' 'omg test-version'
EOF
chmod +x "$package_dir/omg"

tar -czf "$release_dir/omg-linux-amd64.tar.gz" -C "$package_dir" omg
archive_hash=$(sha256sum "$release_dir/omg-linux-amd64.tar.gz" | awk '{ print $1 }')
printf '%s  dist/omg-linux-amd64.tar.gz\n' "$archive_hash" > "$release_dir/SHA256SUMS"

output=$(
  PATH="$fake_bin:$PATH" \
  OMG_DOWNLOAD_BASE_URL="$release_dir" \
  OMG_INSTALL_DIR="$install_dir" \
    sh "$PWD/install.sh"
)

test -x "$install_dir/omg"
test "$($install_dir/omg)" = "omg test-version"
printf '%s\n' "$output" | grep -F "Installed omg to $install_dir/omg" >/dev/null

printf '%064d  omg-linux-amd64.tar.gz\n' 0 > "$release_dir/SHA256SUMS"
if PATH="$fake_bin:$PATH" OMG_DOWNLOAD_BASE_URL="$release_dir" OMG_INSTALL_DIR="$install_dir" sh "$PWD/install.sh" >/dev/null 2>&1; then
  echo "installer accepted an invalid checksum" >&2
  exit 1
fi

echo "Linux installer tests passed"
