#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/openspec-drain-audit.sh [--repo-root PATH] [--changes-root PATH] [--archive PATH]...

Audit OpenSpec drain completion postconditions:
  - no active change directories remain outside archive/ and _done/
  - openspec/changes/.drain-state.md is absent
  - each --archive path exists and is under openspec/changes/archive/

This command is intended for the final drain-complete evidence transcript after
completed changes have been archived.
EOF
}

repo_root=""
changes_root="openspec/changes"
archives=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --repo-root)
      repo_root="${2:-}"
      shift 2
      ;;
    --changes-root)
      changes_root="${2:-}"
      shift 2
      ;;
    --archive)
      archives+=("${2:-}")
      shift 2
      ;;
    *)
      echo "error: unexpected argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$repo_root" ]]; then
  repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    echo "error: not inside a git repository; pass --repo-root" >&2
    exit 2
  }
fi

cd "$repo_root"

if [[ ! -d "$changes_root" ]]; then
  echo "FAIL: changes root does not exist: $changes_root" >&2
  exit 1
fi

printf 'OpenSpec drain audit\n'
printf 'repo_root: %s\n' "$repo_root"
printf 'changes_root: %s\n' "$changes_root"

state_path="$changes_root/.drain-state.md"
if [[ -e "$state_path" ]]; then
  echo "FAIL: drain state file remains: $state_path" >&2
  exit 1
fi
printf 'drain_state: absent (%s)\n' "$state_path"

active_dirs=()
while IFS= read -r dir; do
  base=$(basename "$dir")
  case "$base" in
    archive|_done|.*)
      continue
      ;;
  esac
  active_dirs+=("$dir")
done < <(find "$changes_root" -mindepth 1 -maxdepth 1 -type d | sort)

if [[ ${#active_dirs[@]} -gt 0 ]]; then
  echo "FAIL: active OpenSpec change directories remain:" >&2
  printf '  %s\n' "${active_dirs[@]}" >&2
  exit 1
fi
printf 'active_changes: none\n'

if [[ ${#archives[@]} -eq 0 ]]; then
  printf 'archives_checked: none requested\n'
else
  printf 'archives_checked: %s\n' "${#archives[@]}"
fi

for archive in "${archives[@]}"; do
  archive="${archive#./}"
  case "$archive" in
    "$changes_root"/archive/*) ;;
    *)
      echo "FAIL: archive path is not under $changes_root/archive/: $archive" >&2
      exit 1
      ;;
  esac
  if [[ ! -d "$archive" ]]; then
    echo "FAIL: archive path does not exist: $archive" >&2
    exit 1
  fi
  printf 'archive_path: present (%s)\n' "$archive"
done

printf 'OK: OpenSpec drain archive cleanliness audit passed\n'
