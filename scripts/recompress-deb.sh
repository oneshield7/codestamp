#!/usr/bin/env bash
# Tauri packs the .deb with gzip and has no setting for it; xz makes the package about half the
# size. The files inside stay the same.
set -euo pipefail

bundle="${1:-src-tauri/target/release/bundle/deb}"
shopt -s nullglob
packages=("$bundle"/*.deb)
if [ ${#packages[@]} -eq 0 ]; then
  echo "No .deb in $bundle" >&2
  exit 1
fi

for deb in "${packages[@]}"; do
  work=$(mktemp -d)
  dpkg-deb --raw-extract "$deb" "$work/tree"
  dpkg-deb --root-owner-group -Zxz -z9 -Sextreme --build "$work/tree" "$work/package.deb" > /dev/null
  mv "$work/package.deb" "$deb"
  rm -rf "$work"
  echo "$(basename "$deb"): $(stat -c %s "$deb") bytes"
done
