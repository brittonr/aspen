#!/usr/bin/env bash
# Deploy aspen cluster nodes via nixos-rebuild
# Usage: ./deploy.sh [gmk1|gmk2|gmk3|all]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/framework}"

declare -A HOSTS=(
  [gmk1]="192.168.1.146"
  [gmk2]="192.168.1.114"
  [gmk3]="192.168.1.40"
)

deploy_node() {
  local node="$1"
  local host="${HOSTS[$node]}"
  echo "=== Deploying $node ($host) ==="

  NIX_SSHOPTS="-i $SSH_KEY -o StrictHostKeyChecking=accept-new" \
    nixos-rebuild switch \
      --flake "$SCRIPT_DIR#$node" \
      --option post-build-hook "" \
      --option sandbox false \
      --target-host "root@$host" \
      2>&1

  echo "=== $node done ==="
}

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 [gmk1|gmk2|gmk3|all]"
  exit 1
fi

if [[ "$1" == "all" ]]; then
  for node in gmk1 gmk2 gmk3; do
    deploy_node "$node"
  done
else
  for node in "$@"; do
    if [[ -z "${HOSTS[$node]:-}" ]]; then
      echo "Unknown node: $node (expected gmk1, gmk2, gmk3)"
      exit 1
    fi
    deploy_node "$node"
  done
fi
