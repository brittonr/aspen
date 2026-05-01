#!/usr/bin/env bash
set -euo pipefail

change="openspec/changes/decompose-next-five-crate-families-implementation"
evidence="$change/evidence"
checker="scripts/check-crate-extraction-readiness.rs"
inventory="docs/crate-extraction.md"
manifest_dir="docs/crate-extraction"
family="foundational-types"
base_policy="docs/crate-extraction/policy.ncl"

run_expect_failure() {
  local label="$1"
  local policy="$2"
  local workdir="$3"
  local expected="$4"
  mkdir -p "$workdir/evidence"
  if [ ! -f "$workdir/verification.md" ]; then
    cat > "$workdir/verification.md" <<'EOF'
# Verification Evidence

## Task Coverage
- Evidence: negative-mutation-fixture
EOF
  fi
  local out_json="$evidence/v1-negative-${label}.json"
  local out_md="$evidence/v1-negative-${label}.md"
  local summary="$evidence/v1-negative-${label}-summary.txt"
  local stderr="$evidence/v1-negative-${label}.stderr"
  set +e
  "$checker" \
    --policy "$policy" \
    --inventory "$inventory" \
    --manifest-dir "$manifest_dir" \
    --candidate-family "$family" \
    --output-json "$out_json" \
    --output-markdown "$out_md" \
    2> "$stderr"
  local status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    echo "expected failure but checker passed" > "$summary"
    exit 1
  fi
  if ! grep -F "$expected" "$out_md" >/dev/null; then
    {
      echo "Expected substring: $expected"
      echo "Actual report:"
      cat "$out_md"
    } > "$summary"
    exit 1
  fi
  {
    echo "Command: $checker --policy $policy --inventory $inventory --manifest-dir $manifest_dir --candidate-family $family --output-json $out_json --output-markdown $out_md"
    echo "Exit status: $status"
    echo "Expected substring: $expected"
    grep -F "$expected" "$out_md"
  } > "$summary"
}

mutate_policy() {
  local kind="$1"
  local target="$2"
  python3 - "$kind" "$base_policy" "$target" <<'PY'
import re, sys
kind, src, dst = sys.argv[1:]
text = open(src).read()
block_pat = r'(foundational_types = \{.*?\n    \},)'
m = re.search(block_pat, text, flags=re.S)
if not m:
    raise SystemExit('missing foundational_types block')
block = m.group(1)
if kind == 'missing-owner':
    block2 = block.replace('owner = "Aspen foundational type maintainers"', 'owner = "owner needed"')
elif kind == 'invalid-readiness':
    block2 = block.replace('readiness_state = "workspace-internal"', 'readiness_state = "publishable from monorepo"')
elif kind == 'missing-forbidden-root':
    block2 = block.replace(', "aspen"', '')
else:
    raise SystemExit(kind)
if block2 == block:
    raise SystemExit(f'mutation had no effect: {kind}')
open(dst, 'w').write(text[:m.start(1)] + block2 + text[m.end(1):])
PY
}

tmp_root=$(mktemp -d)
trap 'rm -rf "$tmp_root"' EXIT

p_missing_owner="$tmp_root/missing-owner.ncl"
mutate_policy missing-owner "$p_missing_owner"
run_expect_failure missing-owner "$p_missing_owner" "$tmp_root/work-missing-owner" 'foundational_types: selected family has unassigned owner'

p_invalid_readiness="$tmp_root/invalid-readiness.ncl"
mutate_policy invalid-readiness "$p_invalid_readiness"
run_expect_failure invalid-readiness "$p_invalid_readiness" "$tmp_root/work-invalid-readiness" 'foundational_types: forbidden readiness state `publishable from monorepo`'

p_missing_forbidden="$tmp_root/missing-forbidden-root.ncl"
mutate_policy missing-forbidden-root "$p_missing_forbidden"
run_expect_failure forbidden-runtime "$p_missing_forbidden" "$tmp_root/work-forbidden-runtime" 'foundational_types: selected family does not forbid root `aspen` by default'

run_expect_failure missing-downstream "$base_policy" "$tmp_root/work-missing-downstream" 'foundational-types: missing downstream fixture evidence'

compat_work="$tmp_root/work-missing-compatibility"
mkdir -p "$compat_work/evidence"
cat > "$compat_work/verification.md" <<'EOF'
# Verification Evidence

## Task Coverage
- Evidence: negative-mutation-fixture
EOF
: > "$compat_work/evidence/foundational-types-downstream-metadata.json"
: > "$compat_work/evidence/foundational-types-forbidden-boundary.txt"
run_expect_failure missing-compatibility "$base_policy" "$compat_work" 'foundational-types: missing compatibility evidence'

cat > "$evidence/v1-negative-mutations-summary.txt" <<'EOF'
- missing owner failed selected-wave readiness checking
- invalid readiness state failed selected-wave readiness checking
- missing root runtime forbidden policy failed selected-wave readiness checking
- missing downstream fixture evidence failed selected-wave readiness checking
- missing compatibility evidence failed selected-wave readiness checking
EOF
