#!/bin/sh
set -eu

[ "$#" -eq 1 ] || {
  echo "usage: $0 DIST_DIR" >&2
  exit 2
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_dir=$(dirname -- "$script_dir")
dist_dir=$1

cp "$repository_dir/install.sh" "$repository_dir/install.ps1" "$dist_dir/"
(
  cd "$dist_dir"
  sha256sum omg-linux-amd64.tar.gz omg-macos-arm64.tar.gz omg-windows-amd64.zip > SHA256SUMS
)
