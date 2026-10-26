#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Dependency and lexical boundary checks, not Rust type checking or a compiler."""
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parent
EXPECTED = {
    "core": set(),
    "application": {"molten-orchestrator-core"},
    "batch-core": {"molten-orchestrator-core", "serde"},
    "batch-application": {"molten-batch-core", "async-trait"},
    "codec": {"molten-batch-core", "serde", "serde_json", "preserves", "blake3"},
}
BLOCKED = re.compile(r"\b(?:std|core)\s*::\s*(?:fs|net|process|env|thread|time)\b|\b(?:tokio|redb|iroh|wasmtime|cap_std)\s*::")

def stripped(source: str) -> str:
    # Approximate only: enough for this authored source, not a Rust parser.
    source = re.sub(r'/\*.*?\*/', '', source, flags=re.S)
    source = re.sub(r'//[^\n]*', '', source)
    return re.sub(r'"(?:[^"\\]|\\.)*"', '""', source)

def check() -> list[str]:
    errors = []
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    members = manifest["workspace"]["members"]
    if set(members) != {"core", "application", "batch-core", "batch-application", "codec", "adapters", "cli"}:
        errors.append("Unexpected workspace member set")
    names = {}
    for member in members:
        data = tomllib.loads((ROOT / member / "Cargo.toml").read_text())
        names[data["package"]["name"]] = member
        if member in EXPECTED and set(data.get("dependencies", {})) != EXPECTED[member]:
            errors.append(f"{member}: dependency direction changed")
        for name, dep in data.get("dependencies", {}).items():
            if isinstance(dep, dict) and "path" in dep:
                target = (ROOT / member / dep["path"]).resolve()
                if not target.is_relative_to(ROOT) or not (target / "Cargo.toml").is_file():
                    errors.append(f"{member}: missing or escaping dependency {name}")
    for member in ("core", "batch-core"):
        if "#![no_std]" not in (ROOT / member / "src/lib.rs").read_text():
            errors.append(f"{member}: no_std missing")
    for member in ("core", "application", "batch-core", "batch-application"):
        for path in (ROOT / member / "src").rglob("*.rs"):
            code = stripped(path.read_text())
            if BLOCKED.search(code):
                errors.append(f"{path.relative_to(ROOT)}: ambient effect/backend import")
    for member in members:
        for path in (ROOT / member / "src").rglob("*.rs"):
            if re.search(r'\b(?:todo|unimplemented)\s*!', stripped(path.read_text())):
                errors.append(f"{path.relative_to(ROOT)}: implementation stub")
    return errors

if __name__ == "__main__":
    problems = check()
    if problems:
        print("\n".join(problems))
        raise SystemExit(1)
    print("PASS: seven-crate dependency graph; pure cores/application have no matched ambient imports; no implementation stubs.")
    print("Scope: lexical and manifest checks only. Rust parsing, type checking and runtime behavior are not established.")
