#!/usr/bin/env bash
# Makes the key that signs the APT repository, stores its private half as the APT_SIGNING_KEY
# secret of the GitHub repository and writes a backup. Run it once and keep the backup offline:
# systems that added the repository only accept updates signed with this key.
set -euo pipefail

backup="${1:?usage: new-apt-signing-key.sh <backup file outside the repository>}"
repo="${CODESTAMP_REPO:-oneshield7/codestamp}"
gh="${GH:-gh}"

if [ -e "$backup" ]; then
  echo "$backup already exists" >&2
  exit 1
fi
if git -C "$(dirname "$backup")" rev-parse --is-inside-work-tree > /dev/null 2>&1; then
  echo "Choose a backup path outside any Git repository" >&2
  exit 1
fi

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
trap 'rm -rf "$GNUPGHOME"' EXIT

gpg --batch --quiet --pinentry-mode loopback --passphrase '' \
  --quick-generate-key "Codestamp APT repository" ed25519 sign never
(umask 077 && gpg --batch --armor --export-secret-keys > "$backup")
gpg --batch --armor --export-secret-keys | "$gh" secret set APT_SIGNING_KEY --repo "$repo"
gpg --batch --fingerprint
echo "Private key saved to $backup and stored as the APT_SIGNING_KEY secret of $repo."
