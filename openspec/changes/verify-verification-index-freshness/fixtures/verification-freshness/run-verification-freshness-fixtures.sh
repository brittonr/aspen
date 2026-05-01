#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-openspec-verification-freshness.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

write_fresh_archive() {
  local name="$1"
  local change="openspec/changes/archive/2026-05-01-$name"
  mkdir -p "$TMP_DIR/$change/evidence" "$TMP_DIR/openspec/templates"
  printf 'template\n' > "$TMP_DIR/openspec/templates/verification.md"
  cat > "$TMP_DIR/$change/verification.md" <<EOF
## Implementation Evidence

- Changed file: \`$change/verification.md\`
- Changed file: \`$change/evidence/openspec-preflight-final.txt\`

## Task Coverage

- [x] V1 Fresh fixture
  - Evidence: \`$change/evidence/openspec-preflight-final.txt\`
EOF
  cat > "$TMP_DIR/$change/tasks.md" <<'EOF'
- [x] V1 Fresh fixture
EOF
  sleep 1
  printf 'OK: %s\n' "$name" > "$TMP_DIR/$change/evidence/openspec-preflight-final.txt"
}

expect_pass() {
  local path="$1"
  "$CHECKER" "$TMP_DIR/$path" --repo-root "$TMP_DIR" >/tmp/freshness-check.out 2>&1 || {
    echo "unexpected failure for $path"
    cat /tmp/freshness-check.out
    return 1
  }
  echo "PASS: $path"
}

expect_fail() {
  local path="$1"
  local expected="$2"
  if "$CHECKER" "$TMP_DIR/$path" --repo-root "$TMP_DIR" >/tmp/freshness-check.out 2>&1; then
    echo "unexpected pass for $path"
    cat /tmp/freshness-check.out
    return 1
  fi
  if ! grep -Fq "$expected" /tmp/freshness-check.out; then
    echo "missing expected failure for $path: $expected"
    cat /tmp/freshness-check.out
    return 1
  fi
  echo "PASS: $path rejected: $expected"
}

write_fresh_archive fresh-index
expect_pass openspec/changes/archive/2026-05-01-fresh-index

write_fresh_archive stale-path
sed -i 's#openspec/changes/archive/2026-05-01-stale-path/evidence#openspec/changes/stale-path/evidence#g' "$TMP_DIR/openspec/changes/archive/2026-05-01-stale-path/verification.md"
expect_fail openspec/changes/archive/2026-05-01-stale-path "stale active path"

write_fresh_archive saved-diff
printf 'diff --git a/openspec/changes/saved-diff/file b/openspec/changes/saved-diff/file\n' > "$TMP_DIR/openspec/changes/archive/2026-05-01-saved-diff/evidence/implementation-diff.patch"
cat >> "$TMP_DIR/openspec/changes/archive/2026-05-01-saved-diff/verification.md" <<'EOF'
- Historical diff: `openspec/changes/archive/2026-05-01-saved-diff/evidence/implementation-diff.patch`
EOF
sleep 1
touch "$TMP_DIR/openspec/changes/archive/2026-05-01-saved-diff/evidence/openspec-preflight-final.txt" "$TMP_DIR/openspec/changes/archive/2026-05-01-saved-diff/evidence/implementation-diff.patch"
expect_pass openspec/changes/archive/2026-05-01-saved-diff

write_fresh_archive placeholder-preflight
printf 'pending final preflight\n' > "$TMP_DIR/openspec/changes/archive/2026-05-01-placeholder-preflight/evidence/openspec-preflight-final.txt"
expect_fail openspec/changes/archive/2026-05-01-placeholder-preflight "preflight artifact is placeholder"

write_fresh_archive old-final
sleep 1
printf 'final edit\n' >> "$TMP_DIR/openspec/changes/archive/2026-05-01-old-final/verification.md"
expect_fail openspec/changes/archive/2026-05-01-old-final "final evidence appears older"

write_fresh_archive old-other
sed -i 's#openspec/changes/archive/2026-05-01-old-other/evidence#openspec/changes/some-other-change/evidence#g' "$TMP_DIR/openspec/changes/archive/2026-05-01-old-other/verification.md"
expect_pass openspec/changes/archive/2026-05-01-old-other

echo "verification freshness fixtures passed"
