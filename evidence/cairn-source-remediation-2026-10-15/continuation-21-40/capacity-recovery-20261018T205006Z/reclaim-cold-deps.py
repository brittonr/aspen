#!/usr/bin/env python3
"""One-shot, inode-bound recovery of cold, unaliased Cargo dependency artifacts."""

import argparse
import contextlib
import fcntl
import json
import os
import re
import stat
import subprocess
import time

PROFILE = "/home/brittonr/.cargo-target/repo-targets/seaglass/debug"
EXPECTED_ROOT = {"device": 45, "inode": 2680167, "uid": 1555}
EXPECTED_LOCKS = {
    ".cargo-lock": 2680169,
    ".cargo-build-lock": 2680168,
    ".cargo-artifact-lock": 2680170,
}
ARTIFACT_NAME = re.compile(r"(?:lib)?[A-Za-z0-9_]+-[0-9a-f]{16}(?:\.[A-Za-z0-9_.-]+)?")
ALLOWED_SUFFIXES = {"rlib", "rmeta", "d", "o", "a"}
LSOF = "/etc/profiles/per-user/brittonr/bin/lsof"
B3SUM = "/run/current-system/sw/bin/b3sum"
MAX_ENTRIES = 40000
MAX_ALLOCATED_BYTES = 250 * 1024**3


def identity(value):
    return {"device": value.st_dev, "inode": value.st_ino, "uid": value.st_uid}


def metadata(value):
    return {
        **identity(value),
        "mode": value.st_mode,
        "links": value.st_nlink,
        "logical_bytes": value.st_size,
        "allocated_bytes": value.st_blocks * 512,
        "mtime_ns": value.st_mtime_ns,
        "ctime_ns": value.st_ctime_ns,
    }


@contextlib.contextmanager
def guarded_profile(profile, expected_root, expected_locks):
    """Keep all Cargo lock inodes and the profile directory in place."""
    with contextlib.ExitStack() as stack:
        root = os.open(profile, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        stack.callback(os.close, root)
        if identity(os.fstat(root)) != expected_root:
            raise RuntimeError("Profile identity changed")
        for name, inode in expected_locks.items():
            lock = os.open(name, os.O_RDWR | os.O_NOFOLLOW, dir_fd=root)
            stack.callback(os.close, lock)
            value = os.fstat(lock)
            if (
                identity(value) != {**expected_root, "inode": inode}
                or not stat.S_ISREG(value.st_mode)
                or value.st_nlink != 1
                or value.st_size != 0
            ):
                raise RuntimeError("Cargo lock identity changed: " + name)
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        deps = os.open("deps", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=root)
        stack.callback(os.close, deps)
        value = os.fstat(deps)
        if value.st_dev != expected_root["device"] or value.st_uid != expected_root["uid"]:
            raise RuntimeError("Dependency directory owner or device changed")
        if identity(os.stat(profile, follow_symlinks=False)) != expected_root:
            raise RuntimeError("Profile pathname changed")
        yield root, deps


def inventory(deps, uid, cutoff_ns):
    names = os.listdir(deps)
    if len(names) > MAX_ENTRIES:
        raise RuntimeError("Artifact-count bound exceeded")
    selected = []
    skipped = {}
    device = os.fstat(deps).st_dev
    for name in sorted(names):
        value = os.stat(name, dir_fd=deps, follow_symlinks=False)
        reason = None
        if not ARTIFACT_NAME.fullmatch(name):
            reason = "unrecognized-name"
        elif not stat.S_ISREG(value.st_mode):
            reason = "not-regular"
        elif value.st_dev != device or value.st_uid != uid:
            reason = "different-owner-or-device"
        elif value.st_nlink != 1:
            reason = "hard-linked"
        elif value.st_mtime_ns > cutoff_ns:
            reason = "recent"
        elif "." in name and name.rsplit(".", 1)[1] not in ALLOWED_SUFFIXES:
            reason = "runtime-library-or-unrecognized-suffix"
        elif "." not in name:
            artifact = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=deps)
            try:
                if metadata(os.fstat(artifact)) != metadata(value):
                    raise RuntimeError("Artifact changed while inspecting: " + name)
                if not value.st_mode & 0o111 or os.pread(artifact, 4, 0) != b"\x7fELF":
                    reason = "not-an-elf-executable"
            finally:
                os.close(artifact)
        if reason is not None:
            skipped[reason] = skipped.get(reason, 0) + 1
        else:
            selected.append({"name": name, **metadata(value)})
    allocated = sum(row["allocated_bytes"] for row in selected)
    if allocated > MAX_ALLOCATED_BYTES:
        raise RuntimeError("Allocated-byte bound exceeded")
    return {"entries": selected, "allocated_bytes": allocated, "skipped": skipped}


def usage_observation(profile):
    argv = [LSOF, "-nP", "-a", "-u1555", "-p", "^" + str(os.getpid()), "+D", profile + "/deps", "-Fpcfn"]
    result = subprocess.run(argv, capture_output=True, text=True, timeout=60)
    if result.returncode != 1 or result.stdout:
        raise RuntimeError("Artifact usage was observed or inspection failed: " + result.stdout + result.stderr)
    return {
        "argv": argv,
        "exit_code": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "limit": "UID 1555 observations only; warnings retain visibility limits. The three kernel Cargo locks separately exclude Cargo writers of any UID.",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--apply", metavar="APPROVED_MANIFEST")
    parser.add_argument("--expected-blake3")
    args = parser.parse_args()
    removed = []
    try:
        approved = None
        if args.apply:
            if not args.expected_blake3:
                raise RuntimeError("Apply requires the approved manifest BLAKE3")
            digest = subprocess.run([B3SUM, "--", args.apply], capture_output=True, text=True, check=True).stdout.split()[0]
            if digest != args.expected_blake3:
                raise RuntimeError("Approved manifest identity changed")
            with open(args.apply, encoding="utf-8") as source:
                approved = json.load(source)
            if approved.get("profile") != PROFILE:
                raise RuntimeError("Manifest names a different profile")
        cutoff = approved["cutoff_mtime_ns"] if approved else time.time_ns() - 86400 * 10**9
        with guarded_profile(PROFILE, EXPECTED_ROOT, EXPECTED_LOCKS) as (root, deps):
            observed_usage = usage_observation(PROFILE)
            current = {
                "profile": PROFILE,
                "root_identity": identity(os.fstat(root)),
                "deps_identity": identity(os.fstat(deps)),
                "lock_inodes": EXPECTED_LOCKS,
                "cutoff_mtime_ns": cutoff,
                **inventory(deps, EXPECTED_ROOT["uid"], cutoff),
            }
            if approved is None:
                print(json.dumps({"status": "preview-only", **current, "usage_observation": observed_usage}, indent=2))
                return 0
            for key in ("root_identity", "deps_identity", "lock_inodes", "entries"):
                if current[key] != approved[key]:
                    raise RuntimeError("Approved artifact inventory changed: " + key)
            for entry in current["entries"]:
                artifact = os.open(entry["name"], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=deps)
                try:
                    expected = {key: value for key, value in entry.items() if key != "name"}
                    if metadata(os.fstat(artifact)) != expected:
                        raise RuntimeError("Artifact changed before unlink: " + entry["name"])
                    if metadata(os.stat(entry["name"], dir_fd=deps, follow_symlinks=False)) != expected:
                        raise RuntimeError("Artifact pathname changed: " + entry["name"])
                    os.unlink(entry["name"], dir_fd=deps)
                    removed.append(entry["name"])
                finally:
                    os.close(artifact)
            os.fsync(deps)
            print(json.dumps({"status": "applied", "profile": PROFILE, "removed_names": removed, "allocated_bytes_unlinked": current["allocated_bytes"], "usage_observation": observed_usage, "limit": "Allocated bytes are not a substitute for measured post-operation dataset-available capacity; no protection against arbitrary non-Cargo manual mutators is claimed."}, indent=2))
            return 0
    except Exception as error:
        print(json.dumps({"status": "abstained" if not removed else "partial-failure", "error": str(error), "removed_names": removed}, indent=2))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
