#!/usr/bin/env bash
# Copies the canonical cloud-init recipe from gheiler/artcraft-cloud into the server's template.
#   scripts/sync-cloud-init.sh            # from GitHub (main)
#   scripts/sync-cloud-init.sh ../artcraft-cloud   # from a local checkout
set -euo pipefail
cd "$(dirname "$0")/.."
dest=crates/server/templates/artcraft-cloud.yaml
if [ $# -ge 1 ]; then
  cp "$1/deploy/cloud-init/artcraft-cloud.yaml" "$dest"
else
  curl -fsSL https://raw.githubusercontent.com/gheiler/artcraft-cloud/main/deploy/cloud-init/artcraft-cloud.yaml -o "$dest"
fi
git diff --stat -- "$dest"
echo "Run cargo test -p server to check the template still has every .env key."
