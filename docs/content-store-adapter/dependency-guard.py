#!/usr/bin/env python3
"""Check direct optional object-client dependencies; no Cargo execution required."""
import pathlib
import sys
import tomllib

root = pathlib.Path(__file__).resolve().parents[2]
manifest = tomllib.loads((root / "Cargo.toml").read_text())
allowlist = {}
for line in (root / "docs/content-store-adapter/object-storage-dependency-allowlist.tsv").read_text().splitlines():
    if line and not line.startswith("#"):
        name, version, _ = line.split("\t", 2)
        allowlist[name] = version
features = manifest["features"]
deps = manifest["dependencies"]
errors = []
direct_sections = [manifest]
direct_sections.extend(manifest.get("target", {}).values())
for section in direct_sections:
    for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
        for alias, spec in section.get(kind, {}).items():
            name = spec.get("package", alias) if isinstance(spec, dict) else alias
            if name.startswith("aws-sdk") or name in {"aws-config", "object_store"}:
                errors.append(f"unreviewed direct cloud SDK: {name}; requires a named ADR and policy revision")
selected = set()
for feature in features["content-store-object-storage"]:
    if not feature.startswith("dep:"):
        errors.append(f"unreviewed feature edge: {feature}")
        continue
    alias = feature[4:]
    spec = deps[alias]
    if not isinstance(spec, dict):
        errors.append(f"{alias}: object client dependency must be optional and exact-pinned")
        continue
    name = spec.get("package", alias)
    selected.add(name)
    if name not in allowlist or spec.get("version") != "=" + allowlist.get(name, "") or not spec.get("optional", False):
        errors.append(f"{name}: expected optional exact pin from allowlist")
if selected != set(allowlist):
    errors.append("feature dependencies and allowlist differ")
pending = list(features.get("default", []))
default_closure = set()
while pending:
    feature = pending.pop()
    if feature not in default_closure:
        default_closure.add(feature)
        pending.extend(features.get(feature, []))
if "content-store-object-storage" in default_closure or default_closure.intersection(features["content-store-object-storage"]):
    errors.append("object storage must remain opt-in")
if errors:
    sys.exit("\n".join(errors))
print("object-storage direct dependency allowlist matches; transitive dependencies not classified")
