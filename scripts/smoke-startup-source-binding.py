#!/usr/bin/env python3
"""Exercise real normal startup using an independently signed real producer output.

The caller must build/sign all output paths before running this smoke. Negative
copies retain the original binding: the tampered output changes a gate member;
the incomplete-closure output removes gate-compiler-closure. Negative outputs
must be signed only by the explicitly experimental authority used for this run.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


def measured(path):
    result = subprocess.run(["b3sum", "--no-names", str(path)], check=True, capture_output=True, text=True)
    return "blake3:" + result.stdout.strip()


def command(binary, operation, state, expected=0):
    result = subprocess.run([str(binary), "node", operation, "--state-root", str(state)],
                            capture_output=True, text=True, timeout=180)
    if (result.returncode == 0) != (expected == 0):
        raise RuntimeError(f"node {operation} unexpected exit={result.returncode}\n{result.stdout}\n{result.stderr}")
    return result


def protected(state):
    return {path.relative_to(state).as_posix(): path.read_bytes()
            for path in sorted(state.rglob("*"))
            if path.is_file() and path.name != "source-gate-trust.json"}


def run(args):
    output = args.output.resolve(strict=True)
    executable = output / "bin/molten"
    key = args.trusted_public_key_file.read_text().strip()
    trust = dict(schema="molten-node-source-trust-v1", nix=str(args.nix.resolve(strict=True)),
                 output=str(output), public_key=key,
                 selection=dict(binding_ref=measured(output / "binding.json"),
                                policy_ref=measured(output / "policy"), profile_ref=measured(output / "profile")))
    with tempfile.TemporaryDirectory(prefix="molten-real-source-smoke-", dir=args.workspace_parent) as scratch:
        root = Path(scratch)
        state = root / "state"
        init = subprocess.run([str(executable), "node", "init", "--state-root", str(state),
                               "--node-id", "node:source-binding-smoke"], capture_output=True, text=True, timeout=180)
        if init.returncode != 0:
            raise RuntimeError(f"real node init failed: {init.stdout}\n{init.stderr}")
        trust_path = state / "source-gate-trust.json"
        def select(value):
            trust_path.write_text(json.dumps(value))
        select(trust)
        command(executable, "run", state)
        startup = (state / "startup-receipt.preserves").read_bytes()
        if trust["selection"]["binding_ref"].encode() not in startup:
            raise RuntimeError("normal startup receipt omitted accepted exact binding")
        command(executable, "stop", state)
        before = protected(state)
        for case in ["missing", "stale", "wrong-candidate", "tamper", "missing-gate-closure", "synthetic", "read-error"]:
            selected = json.loads(json.dumps(trust))
            runner = executable
            if case == "stale":
                selected["selection"]["binding_ref"] = "blake3:" + "0" * 64
            elif case == "tamper":
                selected["output"] = str(args.tampered_output.resolve(strict=True))
            elif case == "missing-gate-closure":
                selected["output"] = str(args.incomplete_closure_output.resolve(strict=True))
            elif case == "synthetic":
                selected["schema"] = "explicit-test-fixture"
            elif case == "wrong-candidate":
                runner = root / "wrong-molten"
                shutil.copy2(executable, runner)
                with runner.open("ab") as altered:
                    altered.write(b"\x00wrong-executing-candidate")
            select(selected)
            if case == "missing":
                trust_path.unlink()
            elif case == "read-error":
                trust_path.unlink()
                trust_path.mkdir()
            result = command(runner, "run", state, expected=1)
            expected_issue = {"stale": "StaleSelection", "wrong-candidate": "WrongCandidate",
                              "tamper": "ArtifactMismatch", "synthetic": "unsupported source trust"}.get(case)
            if expected_issue and expected_issue not in result.stdout + result.stderr:
                raise RuntimeError(f"{case} failed outside its required source-admission boundary: {result.stderr}")
            if protected(state) != before:
                raise RuntimeError(f"{case} rejection changed protected lifecycle/evidence bytes")
            print(f"real-normal-startup negative={case} denied exit={result.returncode} lifecycle=unchanged")
            if trust_path.is_dir():
                trust_path.rmdir()
        print("real-normal-startup positive=passed authority=explicitly-experimental production-approval=none")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["output", "tampered-output", "incomplete-closure-output", "nix", "trusted-public-key-file", "workspace-parent"]:
        parser.add_argument("--" + name, type=Path, required=True)
    run(parser.parse_args())


if __name__ == "__main__":
    main()
