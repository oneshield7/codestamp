#!/usr/bin/env bash
# Builds the signed APT repository that GitHub Pages serves: the given .deb files, their package
# index, and a Release file signed with APT_SIGNING_KEY (an armored private key).
set -euo pipefail

out="${1:?usage: apt-repository.sh <output folder> <folder with .deb files>}"
debs="${2:?usage: apt-repository.sh <output folder> <folder with .deb files>}"
: "${APT_SIGNING_KEY:?APT_SIGNING_KEY must hold the armored private key}"

shopt -s nullglob
packages=("$debs"/*.deb)
if [ ${#packages[@]} -eq 0 ]; then
  echo "No .deb in $debs" >&2
  exit 1
fi

suite=dists/stable
index="$suite/main/binary-amd64"
mkdir -p "$out/pool/main" "$out/$index"
cp "${packages[@]}" "$out/pool/main/"
cd "$out"

: > "$index/Packages"
for deb in pool/main/*.deb; do
  {
    dpkg-deb --field "$deb"
    echo "Filename: $deb"
    echo "Size: $(stat -c %s "$deb")"
    echo "SHA256: $(sha256sum "$deb" | cut -d ' ' -f 1)"
    echo
  } >> "$index/Packages"
done
gzip -9 --no-name --keep --force "$index/Packages"

{
  echo "Origin: Codestamp"
  echo "Label: Codestamp"
  echo "Suite: stable"
  echo "Codename: stable"
  echo "Date: $(LC_ALL=C date --utc --rfc-2822)"
  echo "Architectures: amd64"
  echo "Components: main"
  echo "Description: Codestamp, a desktop API client"
  echo "SHA256:"
  for file in main/binary-amd64/Packages main/binary-amd64/Packages.gz; do
    echo " $(sha256sum "$suite/$file" | cut -d ' ' -f 1) $(stat -c %s "$suite/$file") $file"
  done
} > "$suite/Release"

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
trap 'rm -rf "$GNUPGHOME"' EXIT
gpg --batch --quiet --import <<< "$APT_SIGNING_KEY"
gpg --batch --yes --armor --detach-sign --output "$suite/Release.gpg" "$suite/Release"
gpg --batch --yes --clearsign --output "$suite/InRelease" "$suite/Release"
gpg --batch --export > codestamp.gpg

echo "Built $(grep -c '^Package:' "$index/Packages") package(s) in $out"
