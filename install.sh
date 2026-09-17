#!/bin/sh
set -eu

repository="Y-Square-T3/oh-my-gocd"
download_base_url=${OMG_DOWNLOAD_BASE_URL:-"https://github.com/$repository/releases/latest/download"}
install_dir=${OMG_INSTALL_DIR:-"$HOME/.local/bin"}
archive_name="omg-linux-amd64.tar.gz"

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

command -v tar >/dev/null 2>&1 || fail "tar is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

[ "$(uname -s)" = "Linux" ] || fail "this installer supports Linux only"
case "$(uname -m)" in
  x86_64 | amd64) ;;
  *) fail "no prebuilt binary is available for architecture $(uname -m); use cargo install oh-my-gocd" ;;
esac

temp_dir=$(mktemp -d)
trap 'rm -rf "$temp_dir"' EXIT HUP INT TERM

download() {
  source=$1
  destination=$2
  case "$source" in
    http://* | https://*)
      command -v curl >/dev/null 2>&1 || fail "curl is required"
      curl --proto '=https' --tlsv1.2 -fsSL "$source" -o "$destination"
      ;;
    *) cp "$source" "$destination" ;;
  esac
}

download "$download_base_url/$archive_name" "$temp_dir/$archive_name"
download "$download_base_url/SHA256SUMS" "$temp_dir/SHA256SUMS"

expected_hash=$(awk -v name="$archive_name" '$2 == name || $2 == "*" name { print $1; exit }' "$temp_dir/SHA256SUMS")
[ -n "$expected_hash" ] || fail "SHA256SUMS does not contain $archive_name"
actual_hash=$(sha256sum "$temp_dir/$archive_name" | awk '{ print $1 }')
[ "$actual_hash" = "$expected_hash" ] || fail "checksum verification failed for $archive_name"

tar -xzf "$temp_dir/$archive_name" -C "$temp_dir" omg
mkdir -p "$install_dir"
install -m 755 "$temp_dir/omg" "$install_dir/omg"

printf 'Installed omg to %s\n' "$install_dir/omg"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    if [ "$install_dir" = "$HOME/.local/bin" ]; then
      case "${SHELL:-}" in
        */zsh) profile="$HOME/.zshrc" ;;
        */bash) profile="$HOME/.bashrc" ;;
        *) profile="$HOME/.profile" ;;
      esac
      path_line='export PATH="$HOME/.local/bin:$PATH"'
      if ! grep -F "$path_line" "$profile" >/dev/null 2>&1; then
        printf '\n%s\n' "$path_line" >> "$profile"
        printf 'Added %s to PATH in %s; restart your terminal to apply it.\n' "$install_dir" "$profile"
      fi
    else
      printf 'Add %s to PATH to run omg from any directory.\n' "$install_dir"
    fi
    ;;
esac

"$install_dir/omg" --version
