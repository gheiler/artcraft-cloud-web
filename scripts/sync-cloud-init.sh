#!/usr/bin/env bash
# Keeps crates/server/templates/artcraft-cloud.yaml identical to the canonical recipe in
# gheiler/artcraft-cloud (deploy/cloud-init/artcraft-cloud.yaml), so one-click deploys, pasted
# recipes and manual installs all run the same script.
#
#   scripts/sync-cloud-init.sh                         # copy from GitHub, branch main
#   scripts/sync-cloud-init.sh --ref feat/vps-cloud-init
#   scripts/sync-cloud-init.sh ../artcraft-cloud       # copy from a local checkout
#   scripts/sync-cloud-init.sh --check [--ref R | DIR] # only compare; exit 1 if they differ
#
# Exit codes: 0 in sync / copied, 1 differs (--check), 2 the source has no recipe (yet).
set -euo pipefail
cd "$(dirname "$0")/.."
dest=crates/server/templates/artcraft-cloud.yaml
repo=gheiler/artcraft-cloud
ref=main
local_dir=
check=0
while [ $# -gt 0 ]; do
  case "$1" in
    --check) check=1 ;;
    --ref) ref=${2:?--ref needs a branch, tag or commit}; shift ;;
    -h | --help) sed -n '2,12p' "$0"; exit 0 ;;
    *) local_dir=$1 ;;
  esac
  shift
done

src=$(mktemp)
trap 'rm -f "$src"' EXIT
if [ -n "$local_dir" ]; then
  from="$local_dir/deploy/cloud-init/artcraft-cloud.yaml"
  if [ ! -f "$from" ]; then
    echo "No recipe at $from (is that an artcraft-cloud checkout with the cloud-init recipe?)" >&2
    exit 2
  fi
  cp "$from" "$src"
else
  from="$repo@$ref"
  url="https://raw.githubusercontent.com/$repo/$ref/deploy/cloud-init/artcraft-cloud.yaml"
  code=$(curl -sSL -o "$src" -w '%{http_code}' "$url" || true)
  if [ "$code" != 200 ]; then
    echo "deploy/cloud-init/artcraft-cloud.yaml is not on $from (HTTP $code)." >&2
    echo "Until the recipe is merged there, compare with its branch, e.g. --ref feat/vps-cloud-init." >&2
    exit 2
  fi
fi

if cmp -s "$src" "$dest"; then
  echo "In sync with $from (sha256 $(sha256sum "$dest" | cut -c1-12))."
  exit 0
fi
if [ "$check" = 1 ]; then
  echo "$dest differs from $from:" >&2
  diff -u "$dest" "$src" | head -80 >&2 || true
  if [ -n "$local_dir" ]; then again=$local_dir; else again="--ref $ref"; fi
  echo "Run scripts/sync-cloud-init.sh $again to update it." >&2
  exit 1
fi
cp "$src" "$dest"
git diff --stat -- "$dest" || true
echo "Updated from $from. Run cargo test -p server: the template must still have every .env key."
