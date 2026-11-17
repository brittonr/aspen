#!/usr/bin/env python3
"""Import or check Aspen's exact vendored upstream files and bound fixtures.

`vendor/sources.ncl` is the reviewed policy. `import` reads each selected path
from the exact revision of an explicit local checkout and rewrites
`vendor/sources.lock.json`; `check` (the default) verifies the tree against that
lock without network access. Neither mode builds, runs, or trusts upstream code.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

POLICY = "vendor/sources.ncl"
LOCK = "vendor/sources.lock.json"
LOCK_SCHEMA = "molten.vendored-sources-lock.v1"
POLICY_SCHEMA = "molten.vendored-sources.v2"
REGULAR_MODES = {"100644": False, "100755": True}


class VendorError(Exception):
    """A vendored source cannot be imported."""


def git_blob(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def blake3(path: Path, b3sum: str) -> str:
    output = subprocess.run([b3sum, "--no-names", str(path)], check=True, stdout=subprocess.PIPE, text=True).stdout
    return output.strip()


def file_record(path: Path, b3sum: str) -> dict:
    data = path.read_bytes()
    return {"bytes": len(data), "git_blob": git_blob(data), "blake3": blake3(path, b3sum)}


def load_policy(root: Path, nickel: str) -> dict:
    output = subprocess.run(
        [nickel, "export", "--format", "json", POLICY], cwd=root, check=True, stdout=subprocess.PIPE, text=True
    ).stdout
    policy = json.loads(output)
    if policy.get("schema") != POLICY_SCHEMA:
        raise VendorError(f"{POLICY} has schema {policy.get('schema')!r}, expected {POLICY_SCHEMA}")
    return policy


def is_selected(path: str, selected: list[str]) -> bool:
    return any(path == entry or path.startswith(entry + "/") for entry in selected)


# r[impl molten.pure_net.source_intake]
def is_selected_file(source: dict, path: str) -> bool:
    if not is_selected(path, source["selected_paths"]):
        return False
    file_filter = source.get("file_filter")
    return file_filter is None or (
        PurePosixPath(path).suffix in file_filter["extensions"]
        or path in file_filter["exact_paths"]
    )


def is_clean_relative(path: str) -> bool:
    parts = PurePosixPath(path).parts
    return bool(path) and not path.startswith("/") and all(part not in ("", ".", "..") for part in parts)


def check_source(root: Path, source: dict, locked: dict | None, b3sum: str) -> list[str]:
    name = source["name"]
    if locked is None:
        return [f"{name}: missing from {LOCK}"]
    errors = []
    for field in ("repository", "revision", "destination"):
        if locked.get(field) != source[field]:
            errors.append(f"{name}: lock {field} {locked.get(field)!r} differs from policy {source[field]!r}")
    if "policy" in locked and locked["policy"] != source:
        errors.append(f"{name}: locked source policy differs from reviewed policy")
    renamed = {(entry["upstream"], entry["destination"]) for entry in source["renamed_originals"]}
    renamed_upstream = {upstream for upstream, _ in renamed}
    owned = {entry["path"] for entry in source["aspen_owned"]}
    seen_renamed, seen_owned, locked_paths = set(), set(), {}
    for record in locked.get("files", []):
        path, role = record.get("path", ""), record.get("role")
        if not is_clean_relative(path) or path in locked_paths:
            errors.append(f"{name}: invalid or duplicate lock path {path!r}")
            continue
        locked_paths[path] = record
        if role == "upstream":
            if record.get("upstream_path") != path or not is_selected_file(source, path):
                errors.append(f"{name}: {path} is not an unchanged selected upstream path")
            if path in renamed_upstream:
                errors.append(f"{name}: {path} must be renamed, not shipped under its upstream name")
        elif role == "renamed-upstream":
            pair = (record.get("upstream_path"), path)
            if pair not in renamed or not is_selected_file(source, pair[0] or ""):
                errors.append(f"{name}: {path} is not a declared rename of an upstream path")
            seen_renamed.add(pair)
        elif role == "aspen-owned":
            if path not in owned:
                errors.append(f"{name}: {path} is not a declared Aspen-owned file")
            seen_owned.add(path)
        else:
            errors.append(f"{name}: {path} has unknown role {role!r}")
    errors += [f"{name}: declared rename {pair[0]} -> {pair[1]} is missing" for pair in sorted(renamed - seen_renamed)]
    errors += [f"{name}: declared Aspen-owned file {path} is missing" for path in sorted(owned - seen_owned)]
    for selected in source["selected_paths"]:
        covered = any(
            is_selected(record.get("upstream_path") or "", [selected]) for record in locked_paths.values()
        )
        if not covered:
            errors.append(f"{name}: selected path {selected} has no locked file")
    destination = root / source["destination"]
    if not destination.is_dir() or destination.is_symlink():
        return errors + [f"{name}: {source['destination']} is not a directory"]
    on_disk = set()
    for entry in sorted(destination.rglob("*")):
        relative = entry.relative_to(destination).as_posix()
        if entry.is_symlink():
            errors.append(f"{name}: {relative} is a symlink")
        elif entry.is_file():
            on_disk.add(relative)
        elif not entry.is_dir():
            errors.append(f"{name}: {relative} is not a regular file")
    errors += [f"{name}: unexpected file {path}" for path in sorted(on_disk - set(locked_paths))]
    errors += [f"{name}: missing file {path}" for path in sorted(set(locked_paths) - on_disk)]
    for path in sorted(on_disk & set(locked_paths)):
        record = locked_paths[path]
        actual = file_record(destination / path, b3sum)
        for field in ("bytes", "git_blob", "blake3"):
            if actual[field] != record.get(field):
                errors.append(f"{name}: {path} {field} {actual[field]} differs from lock {record.get(field)}")
        executable = bool((destination / path).stat().st_mode & 0o111)
        if executable != bool(record.get("executable", False)):
            errors.append(f"{name}: {path} executable bit differs from lock")
    return errors


def check_notices(root: Path, policy: dict) -> list[str]:
    notices = root / policy["notices"]
    if notices.is_symlink() or not notices.is_file():
        return [f"notice file {policy['notices']} is not a regular file"]
    text = notices.read_text()
    required = [(source["destination"], source["revision"]) for source in policy["sources"]]
    required += [(binding["local_path"], binding["revision"]) for binding in policy["fixture_bindings"]]
    return [
        f"{policy['notices']} lacks a notice naming {path} at revision {revision}"
        for path, revision in required
        if path not in text or revision not in text
    ]


def check_binding(root: Path, binding: dict, locked: dict | None, b3sum: str) -> list[str]:
    label = binding["local_path"]
    if locked is None:
        return [f"fixture {label}: missing from {LOCK}"]
    errors = [
        f"fixture {label}: lock {field} differs from policy"
        for field in ("repository", "revision", "upstream_path")
        if locked.get(field) != binding[field]
    ]
    path = root / label
    if path.is_symlink() or not path.is_file():
        return errors + [f"fixture {label}: not a regular file"]
    actual = file_record(path, b3sum)
    for field in ("bytes", "git_blob", "blake3"):
        if actual[field] != locked.get(field):
            errors.append(f"fixture {label}: {field} {actual[field]} differs from upstream lock {locked.get(field)}")
    return errors


def check_tree(root: Path, policy: dict, lock: dict, b3sum: str) -> list[str]:
    if lock.get("schema") != LOCK_SCHEMA:
        return [f"{LOCK} has schema {lock.get('schema')!r}, expected {LOCK_SCHEMA}"]
    errors = check_notices(root, policy)
    locked_sources = {source.get("name"): source for source in lock.get("sources", [])}
    policy_names = {source["name"] for source in policy["sources"]}
    errors += [f"{name}: locked source is not in policy" for name in sorted(set(locked_sources) - policy_names)]
    if len(locked_sources) != len(lock.get("sources", [])):
        errors.append("duplicate locked source name")
    for source in policy["sources"]:
        errors += check_source(root, source, locked_sources.get(source["name"]), b3sum)
    locked_bindings = {binding.get("local_path"): binding for binding in lock.get("fixture_bindings", [])}
    policy_paths = {binding["local_path"] for binding in policy["fixture_bindings"]}
    errors += [f"fixture {path}: locked binding is not in policy" for path in sorted(set(locked_bindings) - policy_paths)]
    if len(locked_bindings) != len(lock.get("fixture_bindings", [])):
        errors.append("duplicate locked fixture path")
    for binding in policy["fixture_bindings"]:
        errors += check_binding(root, binding, locked_bindings.get(binding["local_path"]), b3sum)
    return errors


def git(checkout: Path, *args: str, binary: bool = False):
    result = subprocess.run(["git", "-C", str(checkout), *args], check=True, stdout=subprocess.PIPE)
    return result.stdout if binary else result.stdout.decode()


def require_commit(checkout: Path, revision: str) -> None:
    try:
        git(checkout, "cat-file", "-e", f"{revision}^{{commit}}")
    except subprocess.CalledProcessError as error:
        raise VendorError(f"{checkout} does not contain commit {revision}") from error


def upstream_entries(checkout: Path, source: dict) -> dict[str, tuple[str, str]]:
    listing = git(checkout, "ls-tree", "-r", "-z", "--full-tree", source["revision"], "--", *source["selected_paths"])
    entries = {}
    for row in filter(None, listing.split("\0")):
        meta, path = row.split("\t", 1)
        mode, kind, obj = meta.split()
        if kind != "blob" or mode not in REGULAR_MODES:
            raise VendorError(f"{source['name']}: {path} is {kind} mode {mode}, not a regular file")
        if not is_selected_file(source, path):
            continue
        entries[path] = (mode, obj)
    for selected in source["selected_paths"]:
        if not any(is_selected(path, [selected]) for path in entries):
            raise VendorError(f"{source['name']}: selected path {selected} is absent at {source['revision']}")
    return entries


def extract(checkout: Path, source: dict, stage: Path) -> None:
    archive = git(checkout, "archive", "--format=tar", source["revision"], "--", *source["selected_paths"], binary=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            if member.isdir():
                continue
            if not member.isfile() or not is_clean_relative(member.name):
                raise VendorError(f"{source['name']}: refusing archive member {member.name!r}")
            if not is_selected_file(source, member.name):
                continue
            target = stage / member.name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(tar.extractfile(member).read())
            target.chmod(0o755 if member.mode & 0o111 else 0o644)


def import_source(root: Path, source: dict, checkout: Path, b3sum: str) -> dict:
    require_commit(checkout, source["revision"])
    entries = upstream_entries(checkout, source)
    destination = root / source["destination"]
    renames = {entry["upstream"]: entry["destination"] for entry in source["renamed_originals"]}
    owned = [entry["path"] for entry in source["aspen_owned"]]
    for path in owned:
        if not (destination / path).is_file():
            raise VendorError(f"{source['name']}: Aspen-owned file {path} must exist before import")
    destination.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=f".import-{source['name']}-", dir=destination.parent))
    try:
        extract(checkout, source, stage)
        for upstream, renamed in renames.items():
            if upstream not in entries:
                raise VendorError(f"{source['name']}: declared rename source {upstream} is absent")
            (stage / renamed).parent.mkdir(parents=True, exist_ok=True)
            (stage / upstream).replace(stage / renamed)
        for path in owned:
            shutil.copyfile(destination / path, stage / path)
        files = []
        for upstream, (mode, obj) in sorted(entries.items()):
            path = renames.get(upstream, upstream)
            record = file_record(stage / path, b3sum)
            if record["git_blob"] != obj:
                raise VendorError(f"{source['name']}: extracted {upstream} does not match blob {obj}")
            role = "renamed-upstream" if upstream in renames else "upstream"
            files.append({"path": path, "upstream_path": upstream, "role": role,
                          "executable": REGULAR_MODES[mode], **record})
        for path in owned:
            files.append({"path": path, "upstream_path": None, "role": "aspen-owned", "executable": False,
                          **file_record(stage / path, b3sum)})
        backup = None
        if destination.exists():
            backup = destination.with_name(f".previous-{destination.name}")
            destination.replace(backup)
        stage.replace(destination)
        if backup is not None:
            shutil.rmtree(backup)
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    keys = ("name", "repository", "revision", "destination")
    return {**{key: source[key] for key in keys}, "policy": source, "files": sorted(files, key=lambda record: record["path"])}


def import_binding(root: Path, binding: dict, checkout: Path, b3sum: str) -> dict:
    require_commit(checkout, binding["revision"])
    upstream = git(checkout, "rev-parse", f"{binding['revision']}:{binding['upstream_path']}").strip()
    record = file_record(root / binding["local_path"], b3sum)
    if record["git_blob"] != upstream:
        raise VendorError(f"fixture {binding['local_path']} differs from upstream blob {upstream}")
    keys = ("repository", "revision", "upstream_path", "local_path")
    return {**{key: binding[key] for key in keys}, **record}


def parse_pairs(values: list[str], label: str) -> dict[str, Path]:
    pairs = {}
    for value in values:
        key, separator, path = value.partition("=")
        if not separator or not key or not path:
            raise VendorError(f"{label} must be KEY=CHECKOUT, got {value!r}")
        pairs[key] = Path(path)
    return pairs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("mode", nargs="?", choices=("check", "import"), default="check")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--source", action="append", default=[], metavar="NAME=CHECKOUT")
    parser.add_argument("--fixture-repository", action="append", default=[], metavar="REPOSITORY=CHECKOUT")
    parser.add_argument("--nickel", default="nickel")
    parser.add_argument("--b3sum", default="b3sum")
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        policy = load_policy(root, args.nickel)
        if args.mode == "import":
            checkouts = parse_pairs(args.source, "--source")
            repositories = parse_pairs(args.fixture_repository, "--fixture-repository")
            old = json.loads((root / LOCK).read_text()) if (root / LOCK).exists() else {
                "schema": LOCK_SCHEMA, "sources": [], "fixture_bindings": []
            }
            known = {s["name"] for s in policy["sources"]}
            known_repositories = {b["repository"] for b in policy["fixture_bindings"]}
            if set(checkouts) - known or set(repositories) - known_repositories:
                raise VendorError("import names an unknown policy source or fixture repository")
            # Validate every preserved row before modifying any destination. A partial
            # import is not permission to regenerate/rebind unavailable old sources.
            preserved_policy = {**policy,
                "sources": [s for s in policy["sources"] if s["name"] not in checkouts],
                "fixture_bindings": [b for b in policy["fixture_bindings"] if b["repository"] not in repositories]}
            preserved_lock = {**old,
                "sources": [s for s in old["sources"] if s["name"] not in checkouts],
                "fixture_bindings": [b for b in old["fixture_bindings"] if b["repository"] not in repositories]}
            # Legacy v1 locks omitted full reviewed policy rows. Anchor those
            # preserved rows to this repository's committed policy AND lock;
            # tree hashes alone cannot detect silently broadened selection or
            # changed license/disclosure. No upstream checkout is needed.
            if preserved_policy["sources"] or preserved_policy["fixture_bindings"]:
                baseline_text = git(root, "show", f"HEAD:{POLICY}")
                baseline_policy = json.loads(subprocess.run(
                    [args.nickel, "export", "--format", "json"], input=baseline_text,
                    check=True, stdout=subprocess.PIPE, text=True).stdout)
                baseline_lock = json.loads(git(root, "show", f"HEAD:{LOCK}"))
                for group, key in (("sources", "name"), ("fixture_bindings", "local_path")):
                    baseline_rows = {row[key]: row for row in baseline_policy[group]}
                    baseline_locked = {row[key]: row for row in baseline_lock[group]}
                    preserved_rows = {row[key]: row for row in preserved_lock[group]}
                    for row in preserved_policy[group]:
                        if baseline_rows.get(row[key]) != row or baseline_locked.get(row[key]) != preserved_rows.get(row[key]):
                            raise VendorError(f"preserved {group} row {row[key]} differs from committed policy/lock; provide its exact checkout for deliberate import")
            errors = check_tree(root, preserved_policy, preserved_lock, args.b3sum)
            if errors:
                raise VendorError("preserved source validation failed: " + "; ".join(errors))
            previous_sources = {s["name"]: s for s in old["sources"]}
            previous_bindings = {b["local_path"]: b for b in old["fixture_bindings"]}
            lock = {
                "schema": LOCK_SCHEMA,
                "sources": [import_source(root, s, checkouts[s["name"]], args.b3sum)
                            if s["name"] in checkouts else previous_sources[s["name"]]
                            for s in policy["sources"]],
                "fixture_bindings": [import_binding(root, b, repositories[b["repository"]], args.b3sum)
                                     if b["repository"] in repositories else previous_bindings[b["local_path"]]
                                     for b in policy["fixture_bindings"]],
            }
            (root / LOCK).write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n")
        lock = json.loads((root / LOCK).read_text())
        errors = check_tree(root, policy, lock, args.b3sum)
    except (VendorError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"vendored sources: {error}", file=sys.stderr)
        return 1
    for error in errors:
        print(f"vendored sources: {error}", file=sys.stderr)
    if errors:
        return 1
    files = sum(len(source["files"]) for source in lock["sources"])
    print(f"vendored sources: {files} files and {len(lock['fixture_bindings'])} fixture bindings match {LOCK}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
