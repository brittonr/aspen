#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." >/dev/null && pwd)
PREFLIGHT="$REPO_ROOT/scripts/openspec-preflight.sh"
DETAIL_OUTPUT="${OPENSPEC_PREFLIGHT_SHOW_CASE_OUTPUT:-0}"

readonly SUCCESS_STATUS=0
readonly FAILURE_STATUS=1
readonly TASK_TEXT="I1 Fixture checked task proves durable evidence coverage. [covers=fixture.checked-task]"
readonly ARCHIVE_PREFIX="2026-04-29"

TMP_ROOT=""
cleanup() {
  if [[ -n "$TMP_ROOT" && -d "$TMP_ROOT" ]]; then
    rm -rf -- "$TMP_ROOT"
  fi
}
trap cleanup EXIT

init_fixture_repo() {
  local repo_dir="$1"
  mkdir -p -- "$repo_dir"
  git -C "$repo_dir" init -q
  git -C "$repo_dir" config user.email "openspec-preflight-fixture@example.invalid"
  git -C "$repo_dir" config user.name "OpenSpec Preflight Fixture"
}

write_tasks() {
  local change_dir="$1"
  cat > "$change_dir/tasks.md" <<EOF_TASKS
## 1. Implementation

- [x] $TASK_TEXT
EOF_TASKS
}

write_common_files() {
  local repo_dir="$1"
  local change_rel="$2"
  local evidence_path="$3"
  local changed_file="$4"
  local artifact_path="$change_rel/evidence/preflight-fixture.txt"
  local change_dir="$repo_dir/$change_rel"

  mkdir -p -- "$change_dir/evidence" "$repo_dir/docs" "$repo_dir/src"
  write_tasks "$change_dir"

  cat > "$repo_dir/$artifact_path" <<'EOF_ARTIFACT'
preflight fixture artifact: command output captured for durable evidence validation
EOF_ARTIFACT

  cat > "$repo_dir/$changed_file" <<'EOF_CHANGED'
Current changed source/doc evidence used by the fixture.
EOF_CHANGED

  cat > "$change_dir/verification.md" <<EOF_VERIFY
# Verification Evidence

## Implementation Evidence

- Changed file: \`$changed_file\`

## Task Coverage

- [x] $TASK_TEXT
  - Evidence: \`$evidence_path\`

## Verification Commands

### \`fixture command\`

- Status: pass
- Artifact: \`$artifact_path\`
EOF_VERIFY
}

stage_fixture_repo() {
  local repo_dir="$1"
  git -C "$repo_dir" add -A
}

make_active_change_rel() {
  local case_name="$1"
  printf 'openspec/changes/%s\n' "$case_name"
}

make_archived_change_rel() {
  local case_name="$1"
  printf 'openspec/changes/archive/%s-%s\n' "$ARCHIVE_PREFIX" "$case_name"
}

setup_valid_change_local() {
  local repo_dir="$1"
  local change_rel="$2"
  local evidence_path="$change_rel/evidence/task-proof.txt"
  write_common_files "$repo_dir" "$change_rel" "$evidence_path" "docs/current.md"
  cat > "$repo_dir/$evidence_path" <<'EOF_EVIDENCE'
Valid change-local evidence: checked task cites a tracked file inside the change evidence directory.
EOF_EVIDENCE
  stage_fixture_repo "$repo_dir"
}

setup_valid_changed_file() {
  local repo_dir="$1"
  local change_rel="$2"
  write_common_files "$repo_dir" "$change_rel" "docs/current.md" "docs/current.md"
  stage_fixture_repo "$repo_dir"
}

setup_missing_evidence() {
  local repo_dir="$1"
  local change_rel="$2"
  local change_dir="$repo_dir/$change_rel"
  mkdir -p -- "$change_dir/evidence" "$repo_dir/docs"
  write_tasks "$change_dir"
  cat > "$repo_dir/docs/current.md" <<'EOF_CHANGED'
Current changed doc exists, but task coverage has no evidence line.
EOF_CHANGED
  cat > "$change_dir/evidence/preflight-fixture.txt" <<'EOF_ARTIFACT'
artifact exists, task evidence missing
EOF_ARTIFACT
  cat > "$change_dir/verification.md" <<EOF_VERIFY
# Verification Evidence

## Implementation Evidence

- Changed file: \`docs/current.md\`

## Task Coverage

- [x] $TASK_TEXT

## Verification Commands

### \`fixture command\`

- Status: pass
- Artifact: \`$change_rel/evidence/preflight-fixture.txt\`
EOF_VERIFY
  stage_fixture_repo "$repo_dir"
}

setup_untracked_evidence() {
  local repo_dir="$1"
  local change_rel="$2"
  local evidence_path="$change_rel/evidence/untracked-proof.txt"
  write_common_files "$repo_dir" "$change_rel" "$evidence_path" "docs/current.md"
  cat > "$repo_dir/$evidence_path" <<'EOF_EVIDENCE'
This file exists but remains untracked to prove path-specific failure output.
EOF_EVIDENCE
  stage_fixture_repo "$repo_dir"
  git -C "$repo_dir" reset -q -- "$evidence_path"
}

setup_empty_evidence() {
  local repo_dir="$1"
  local change_rel="$2"
  local evidence_path="$change_rel/evidence/empty-proof.txt"
  write_common_files "$repo_dir" "$change_rel" "$evidence_path" "docs/current.md"
  : > "$repo_dir/$evidence_path"
  stage_fixture_repo "$repo_dir"
}

setup_placeholder_evidence() {
  local repo_dir="$1"
  local change_rel="$2"
  local placeholder_text="$3"
  local evidence_path="$change_rel/evidence/placeholder-proof.txt"
  write_common_files "$repo_dir" "$change_rel" "$evidence_path" "docs/current.md"
  printf '%s\n' "$placeholder_text" > "$repo_dir/$evidence_path"
  stage_fixture_repo "$repo_dir"
}

setup_disallowed_external_evidence() {
  local repo_dir="$1"
  local change_rel="$2"
  write_common_files "$repo_dir" "$change_rel" "docs/external-proof.md" "docs/current.md"
  cat > "$repo_dir/docs/external-proof.md" <<'EOF_EVIDENCE'
Tracked external evidence exists but is neither change-local nor listed as implementation evidence.
EOF_EVIDENCE
  stage_fixture_repo "$repo_dir"
}

run_case() {
  local case_name="$1"
  local change_rel="$2"
  local expected_status="$3"
  local allow_untracked="$4"
  local required_reason="$5"
  local expected_evidence_path="$6"
  local repo_dir="$TMP_ROOT/$case_name"
  local stdout_path="$TMP_ROOT/$case_name.stdout"
  local stderr_path="$TMP_ROOT/$case_name.stderr"
  local combined_path="$TMP_ROOT/$case_name.combined"

  shift 6
  local setup_fn="$1"
  shift
  init_fixture_repo "$repo_dir"
  "$setup_fn" "$repo_dir" "$change_rel" "$@"

  local display_command="$PREFLIGHT $change_rel"
  set +e
  if [[ "$allow_untracked" == "yes" ]]; then
    display_command="OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1 $display_command"
    (cd "$repo_dir" && OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1 "$PREFLIGHT" "$change_rel") >"$stdout_path" 2>"$stderr_path"
  else
    (cd "$repo_dir" && "$PREFLIGHT" "$change_rel") >"$stdout_path" 2>"$stderr_path"
  fi
  local actual_status=$?
  set -e

  {
    printf '$ %s\n' "$display_command"
    printf 'expected_status=%s actual_status=%s\n' "$expected_status" "$actual_status"
    printf -- '--- stdout ---\n'
    cat "$stdout_path"
    printf -- '--- stderr ---\n'
    cat "$stderr_path"
  } > "$combined_path"
  if [[ "$actual_status" -ne "$expected_status" ]]; then
    echo "FAIL: $case_name expected status $expected_status got $actual_status" >&2
    cat "$combined_path" >&2
    return "$FAILURE_STATUS"
  fi

  if [[ "$expected_status" -ne "$SUCCESS_STATUS" ]]; then
    grep -Fq "$TASK_TEXT" "$combined_path" || {
      echo "FAIL: $case_name missing task text in failure output" >&2
      cat "$combined_path" >&2
      return "$FAILURE_STATUS"
    }
    grep -Fq "Remediation:" "$combined_path" || {
      echo "FAIL: $case_name missing remediation in failure output" >&2
      cat "$combined_path" >&2
      return "$FAILURE_STATUS"
    }
    grep -Fq "$required_reason" "$combined_path" || {
      echo "FAIL: $case_name missing expected reason: $required_reason" >&2
      cat "$combined_path" >&2
      return "$FAILURE_STATUS"
    }
    if [[ -n "$expected_evidence_path" ]]; then
      grep -Fq "Evidence: $expected_evidence_path" "$combined_path" || {
        echo "FAIL: $case_name missing expected evidence path: $expected_evidence_path" >&2
        cat "$combined_path" >&2
        return "$FAILURE_STATUS"
      }
    fi
  fi

  if [[ "$DETAIL_OUTPUT" == "1" ]]; then
    printf -- '--- case %s transcript ---\n' "$case_name"
    cat "$combined_path"
    printf -- '--- end case %s transcript ---\n' "$case_name"
  fi

  printf 'PASS %s\n' "$case_name"
}

main() {
  TMP_ROOT=$(mktemp -d)

  run_case "active-change-local" "$(make_active_change_rel active-change-local)" "$SUCCESS_STATUS" "no" "" "" setup_valid_change_local
  run_case "active-changed-file" "$(make_active_change_rel active-changed-file)" "$SUCCESS_STATUS" "no" "" "" setup_valid_changed_file
  run_case "archived-change-local" "$(make_archived_change_rel archived-change-local)" "$SUCCESS_STATUS" "no" "" "" setup_valid_change_local
  run_case "archived-changed-file" "$(make_archived_change_rel archived-changed-file)" "$SUCCESS_STATUS" "no" "" "" setup_valid_changed_file

  run_case "active-missing-evidence" "$(make_active_change_rel active-missing-evidence)" "$FAILURE_STATUS" "no" "task coverage entry has no evidence paths" "" setup_missing_evidence
  run_case "active-untracked-evidence" "$(make_active_change_rel active-untracked-evidence)" "$FAILURE_STATUS" "yes" "task evidence path is not tracked by git" "openspec/changes/active-untracked-evidence/evidence/untracked-proof.txt" setup_untracked_evidence
  run_case "active-empty-evidence" "$(make_active_change_rel active-empty-evidence)" "$FAILURE_STATUS" "no" "evidence file is empty" "openspec/changes/active-empty-evidence/evidence/empty-proof.txt" setup_empty_evidence
  run_case "active-pending-evidence" "$(make_active_change_rel active-pending-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/active-pending-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "pending"
  run_case "active-todo-evidence" "$(make_active_change_rel active-todo-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/active-todo-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "TODO"
  run_case "active-placeholder-evidence" "$(make_active_change_rel active-placeholder-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/active-placeholder-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "placeholder"
  run_case "active-disallowed-external" "$(make_active_change_rel active-disallowed-external)" "$FAILURE_STATUS" "no" "must be change-local or listed as current implementation evidence" "docs/external-proof.md" setup_disallowed_external_evidence

  run_case "archived-missing-evidence" "$(make_archived_change_rel missing-evidence)" "$FAILURE_STATUS" "no" "task coverage entry has no evidence paths" "" setup_missing_evidence
  run_case "archived-untracked-evidence" "$(make_archived_change_rel untracked-evidence)" "$FAILURE_STATUS" "yes" "task evidence path is not tracked by git" "openspec/changes/archive/2026-04-29-untracked-evidence/evidence/untracked-proof.txt" setup_untracked_evidence
  run_case "archived-empty-evidence" "$(make_archived_change_rel empty-evidence)" "$FAILURE_STATUS" "no" "evidence file is empty" "openspec/changes/archive/2026-04-29-empty-evidence/evidence/empty-proof.txt" setup_empty_evidence
  run_case "archived-pending-evidence" "$(make_archived_change_rel pending-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/archive/2026-04-29-pending-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "pending"
  run_case "archived-todo-evidence" "$(make_archived_change_rel todo-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/archive/2026-04-29-todo-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "TODO"
  run_case "archived-placeholder-evidence" "$(make_archived_change_rel placeholder-evidence)" "$FAILURE_STATUS" "no" "placeholder" "openspec/changes/archive/2026-04-29-placeholder-evidence/evidence/placeholder-proof.txt" setup_placeholder_evidence "placeholder"
  run_case "archived-disallowed-external" "$(make_archived_change_rel disallowed-external)" "$FAILURE_STATUS" "no" "must be change-local or listed as current implementation evidence" "docs/external-proof.md" setup_disallowed_external_evidence
}

main "$@"
