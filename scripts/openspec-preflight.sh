#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/openspec-preflight.sh [CHANGE_DIR_OR_NAME]

Checks that checked OpenSpec tasks are backed by durable repo evidence.

Rules enforced:
  - checked tasks in tasks.md must have matching entries in verification.md
  - each checked task must cite repo-relative evidence paths
  - evidence paths must exist, be tracked by git, and contain non-placeholder content
  - task evidence must be change-local or a currently changed file listed in verification.md
  - changed files listed in verification.md must currently appear in git status
  - no untracked files may remain in the repo unless OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1
  - newly added files must be tracked (prevents nix/flake source-filter misses)

If no argument is provided and exactly one active change exists under
openspec/changes/active/, that change is used automatically.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "error: not inside a git repository" >&2
  exit 1
}
cd "$repo_root"

resolve_change_dir() {
  local arg="${1:-}"

  if [[ -z "$arg" ]]; then
    mapfile -t active_dirs < <(find openspec/changes/active -mindepth 1 -maxdepth 1 -type d | sort)
    case "${#active_dirs[@]}" in
      0)
        echo "error: no active OpenSpec change found" >&2
        return 1
        ;;
      1)
        printf '%s\n' "${active_dirs[0]}"
        return 0
        ;;
      *)
        echo "error: multiple active OpenSpec changes found; pass one explicitly:" >&2
        printf '  %s\n' "${active_dirs[@]}" >&2
        return 1
        ;;
    esac
  fi

  if [[ -d "$arg" ]]; then
    printf '%s\n' "${arg#./}"
    return 0
  fi

  if [[ -d "openspec/changes/active/$arg" ]]; then
    printf '%s\n' "openspec/changes/active/$arg"
    return 0
  fi

  if [[ -d "openspec/changes/$arg" ]]; then
    printf '%s\n' "openspec/changes/$arg"
    return 0
  fi

  echo "error: could not resolve OpenSpec change '$arg'" >&2
  return 1
}

change_dir=$(resolve_change_dir "${1:-}")

if [[ -x "$repo_root/scripts/check-openspec-delta-consistency.py" ]]; then
  "$repo_root/scripts/check-openspec-delta-consistency.py" "$change_dir" --repo-root "$repo_root" >/dev/null
fi

if [[ -x "$repo_root/scripts/check-openspec-design-verification.py" ]]; then
  "$repo_root/scripts/check-openspec-design-verification.py" "$change_dir" --repo-root "$repo_root" >/dev/null
fi

python3 - "$repo_root" "$change_dir" <<'PY'
import os
import pathlib
import re
import subprocess
import sys
from typing import Dict, List, Optional

repo_root = pathlib.Path(sys.argv[1]).resolve()
change_dir = pathlib.Path(sys.argv[2]).resolve()
change_dir_rel = change_dir.relative_to(repo_root).as_posix()

tasks_path = change_dir / "tasks.md"
verification_path = change_dir / "verification.md"

PLACEHOLDER_TERMS = ("pending", "todo", "placeholder")
PLACEHOLDER_LINE_RE = re.compile(
    r"^\s*(?:[-*]\s*)?(?:status\s*:\s*)?(pending|todo|placeholder)\s*[.!]?\s*$",
    re.IGNORECASE,
)


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def task_remediation() -> str:
    return (
        "Remediation: copy the checked task verbatim into verification.md under "
        "'## Task Coverage' and add '- Evidence: `repo/path`' pointing to a "
        "tracked change-local evidence artifact or a currently modified/staged "
        "source/doc file listed in '## Implementation Evidence'."
    )


def fail_task(task: str, reason: str, evidence_path: Optional[str] = None) -> None:
    details = [f"checked task evidence is invalid: {reason}", f"Task: {task}"]
    if evidence_path is not None:
        details.append(f"Evidence: {evidence_path}")
    details.append(task_remediation())
    fail("\n  ".join(details))

if not tasks_path.exists():
    fail(f"missing tasks.md: {tasks_path}")

tasks_text = tasks_path.read_text()
checked_tasks = [m.group(1).strip() for m in re.finditer(r"^\s*-\s*\[x\]\s+(.*\S)\s*$", tasks_text, re.MULTILINE)]
all_tasks = [m.group(1).strip() for m in re.finditer(r"^\s*-\s*\[[ x]\]\s+(.*\S)\s*$", tasks_text, re.MULTILINE)]

if not checked_tasks:
    print(f"OK: no checked tasks in {change_dir_rel}/tasks.md")
    raise SystemExit(0)

if not verification_path.exists():
    fail(
        f"checked tasks require verification.md: {verification_path}\n"
        + "Checked tasks:\n  - "
        + "\n  - ".join(checked_tasks)
        + "\n"
        + task_remediation()
    )

verification_text = verification_path.read_text()
changed_files = [m.group(1).strip() for m in re.finditer(r"^\s*-\s*Changed file:\s*`([^`]+)`\s*$", verification_text, re.MULTILINE)]
if not changed_files:
    fail("verification.md must list at least one changed file under '## Implementation Evidence'")

coverage_section_match = re.search(r"^## Task Coverage\s*$\n(.*?)(?=^##\s|\Z)", verification_text, re.MULTILINE | re.DOTALL)
if not coverage_section_match:
    fail("verification.md must contain a '## Task Coverage' section")
coverage_section = coverage_section_match.group(1)

coverage: Dict[str, List[str]] = {}
current_task = None
for raw_line in coverage_section.splitlines():
    task_match = re.match(r"^\s*-\s*\[x\]\s+(.*\S)\s*$", raw_line)
    if task_match:
        current_task = task_match.group(1).strip()
        coverage[current_task] = []
        continue
    evidence_match = re.match(r"^\s*-\s*Evidence:\s*(.*\S)?\s*$", raw_line)
    if evidence_match and current_task is not None:
        coverage[current_task].extend(re.findall(r"`([^`]+)`", evidence_match.group(1) or ""))

missing = [task for task in checked_tasks if task not in coverage]
extra = [task for task in coverage if task not in checked_tasks]
if missing:
    fail(
        "verification.md is missing checked task coverage entries for:\n  - "
        + "\n  - ".join(missing)
        + "\n"
        + task_remediation()
    )
if extra:
    fail("verification.md has task coverage entries for unchecked tasks:\n  - " + "\n  - ".join(extra))

def is_tracked(path: str) -> bool:
    result = subprocess.run(
        ["git", "ls-files", "--error-unmatch", path],
        cwd=repo_root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def has_status(path: str) -> bool:
    result = subprocess.run(
        ["git", "status", "--porcelain", "--", path],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    return bool(result.stdout.strip())


def list_untracked_paths() -> List[str]:
    result = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        fail("failed to query untracked files from git")
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def normalize_repo_path(path: str, task: Optional[str] = None) -> pathlib.Path:
    candidate = pathlib.Path(path)
    if candidate.is_absolute():
        if task is not None:
            fail_task(task, "evidence path must be repo-relative, not absolute", path)
        fail(f"evidence path must be repo-relative, not absolute: {path}")
    resolved = (repo_root / candidate).resolve()
    try:
        resolved.relative_to(repo_root)
    except ValueError:
        if task is not None:
            fail_task(task, "evidence path escapes repo root", path)
        fail(f"evidence path escapes repo root: {path}")
    return resolved


def is_change_local(path: str) -> bool:
    return path == change_dir_rel or path.startswith(change_dir_rel + "/")


def placeholder_reason(text: str) -> Optional[str]:
    stripped = text.strip()
    if not stripped:
        return "evidence file is empty"
    lower = stripped.lower()
    if lower in PLACEHOLDER_TERMS:
        return f"evidence file contains only placeholder text: {stripped!r}"
    material_lines = [line.strip() for line in stripped.splitlines() if line.strip() and not line.strip().startswith("#")]
    if material_lines and all(PLACEHOLDER_LINE_RE.match(line) for line in material_lines):
        return "evidence file contains only placeholder lines"
    return None


def validate_evidence_content(path: str, resolved: pathlib.Path, task: Optional[str] = None) -> None:
    if resolved.is_dir():
        if task is not None:
            fail_task(task, "evidence path must be a file, not a directory", path)
        fail(f"evidence path must be a file, not a directory: {path}")
    try:
        text = resolved.read_text(errors="replace")
    except OSError as exc:
        if task is not None:
            fail_task(task, f"could not read evidence file: {exc}", path)
        fail(f"could not read evidence file {path}: {exc}")
    reason = placeholder_reason(text)
    if reason is not None:
        if task is not None:
            fail_task(task, reason, path)
        fail(f"invalid evidence artifact {path}: {reason}")

allow_untracked = os.environ.get("OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED") == "1"
if not allow_untracked:
    untracked_paths = list_untracked_paths()
    if untracked_paths:
        fail(
            "untracked files detected:\n  - "
            + "\n  - ".join(untracked_paths)
            + "\nStage, remove, or ignore them before preflight. Set OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1 to bypass."
        )

for path in changed_files:
    resolved = normalize_repo_path(path)
    if not resolved.exists():
        fail(f"listed changed file does not exist: {path}")
    if not is_tracked(path):
        fail(f"listed changed file is not tracked by git: {path}")
    if not has_status(path):
        fail(f"listed changed file is not currently modified/staged: {path}")

commands_section_match = re.search(r"^## Verification Commands\s*$\n(.*?)(?=^##\s|\Z)", verification_text, re.MULTILINE | re.DOTALL)
if not commands_section_match:
    fail("verification.md must contain a '## Verification Commands' section")
artifact_paths = re.findall(r"^\s*-\s*Artifact:\s*`([^`]+)`\s*$", commands_section_match.group(1), re.MULTILINE)
if not artifact_paths:
    fail("verification.md must list at least one verification artifact under '## Verification Commands'")

for path in artifact_paths:
    resolved = normalize_repo_path(path)
    normalized_path = resolved.relative_to(repo_root).as_posix()
    if not resolved.exists():
        fail(f"verification artifact does not exist: {path}")
    if not is_tracked(normalized_path):
        fail(f"verification artifact is not tracked by git: {path}")
    validate_evidence_content(normalized_path, resolved)

changed_file_set = set(changed_files)
for task, evidence_paths in coverage.items():
    if not evidence_paths:
        fail_task(task, "task coverage entry has no evidence paths")
    for path in evidence_paths:
        resolved = normalize_repo_path(path, task)
        normalized_path = resolved.relative_to(repo_root).as_posix()
        if not resolved.exists():
            fail_task(task, "task evidence path does not exist", path)
        if not is_tracked(normalized_path):
            fail_task(task, "task evidence path is not tracked by git", path)
        if normalized_path not in changed_file_set and not is_change_local(normalized_path):
            fail_task(
                task,
                "task evidence path must be change-local or listed as current implementation evidence",
                path,
            )
        validate_evidence_content(normalized_path, resolved, task)

print(f"OK: {change_dir_rel}")
print(f"  tasks: {len(all_tasks)} total / {len(checked_tasks)} checked")
print(f"  changed files listed: {len(changed_files)}")
print(f"  verification artifacts: {len(artifact_paths)}")
PY
