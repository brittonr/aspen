#!/usr/bin/env python3
"""Measure an executed Nix candidate and its retained gate cohort; never sign or self-trust."""
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

MAX_FILES = 4096
MAX_BYTES = 256 * 1024 * 1024


def digest_stream(feed):
    child = subprocess.Popen(["b3sum", "--no-names"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        feed(child.stdin)
        child.stdin.close()
        result = child.stdout.read(256)
        if child.wait() != 0:
            raise RuntimeError("BLAKE3 measurement failed")
        value = result.decode("ascii").strip()
        if len(value) != 64 or any(ch not in "0123456789abcdef" for ch in value):
            raise RuntimeError("invalid BLAKE3 result")
        return "blake3:" + value
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()


def file_ref(path):
    def feed(sink):
        with path.open("rb") as source:
            shutil.copyfileobj(source, sink, length=65536)
    return digest_stream(feed)


def cohort_ref(root):
    members = sorted(root.rglob("*"), key=lambda path: path.relative_to(root).as_posix())
    if len(members) > MAX_FILES:
        raise RuntimeError("gate cohort entry bound exceeded")
    files = []
    total = 0
    for member in members:
        if member.is_symlink() or not (member.is_file() or member.is_dir()):
            raise RuntimeError("gate cohort contains non-regular member")
        if member.is_file():
            total += member.stat().st_size
            files.append(member)
    if total > MAX_BYTES or not files:
        raise RuntimeError("gate cohort byte bound or empty cohort")
    def feed(sink):
        for member in files:
            name = member.relative_to(root).as_posix().encode("utf-8")
            sink.write(struct.pack("<Q", len(name)))
            sink.write(name)
            sink.write(struct.pack("<Q", member.stat().st_size))
            with member.open("rb") as source:
                shutil.copyfileobj(source, sink, length=65536)
    return digest_stream(feed)


def nar_ref(path):
    def feed(sink):
        subprocess.run(["nix-store", "--dump", str(path)], stdout=sink, check=True)
    return digest_stream(feed)


def measured_closure(graphs, output):
    paths = set()
    records = []
    for role, graph in graphs:
        members = set()
        if graph.stat().st_size > 64 * 1024 * 1024:
            raise RuntimeError("build graph byte bound exceeded")
        for line in graph.read_text().splitlines():
            if line.startswith("/nix/store/"):
                path = Path(line)
                if path.parent != Path("/nix/store"):
                    raise RuntimeError("build graph contains a non-root store path")
                members.add(path)
        if not members:
            raise RuntimeError(f"empty {role} dependency graph")
        paths.update(members)
        if len(paths) > 65536:
            raise RuntimeError("build closure path bound exceeded")
        shutil.copy2(graph, output.parent / (role + "-closure"))
        records.append(dict(role=role, graph_ref=file_ref(graph), paths=[str(path) for path in sorted(members)]))
    measured = [{"path": str(path), "nar_ref": nar_ref(path)} for path in sorted(paths)]
    output.write_text(json.dumps({"schema": "molten-measured-build-closure-v1", "graphs": records, "paths": measured},
                                sort_keys=True, separators=(",", ":")) + "\n")


def copy_binaries(candidate, output):
    members = []
    for member in (candidate / "bin").iterdir():
        members.append(member)
        if len(members) > 64:
            raise RuntimeError("candidate binary count bound exceeded")
    required = {"molten", "molten-native-extension-fixture", "molten-collaboration-scope"}
    if not required.issubset({member.name for member in members}):
        raise RuntimeError("candidate is missing a required production-built host binary")
    total = 0
    (output / "bin").mkdir(parents=True)
    for member in sorted(members):
        if member.is_symlink() or not member.is_file():
            raise RuntimeError("candidate bin contains a link or non-regular member")
        metadata = member.stat()
        total += metadata.st_size
        if metadata.st_size > 2 * 1024 ** 3 or total > 8 * 1024 ** 3 or not metadata.st_mode & 0o111:
            raise RuntimeError("candidate binary size or executable-mode bound exceeded")
        with member.open("rb") as executable:
            if executable.read(4) != b"\x7fELF":
                raise RuntimeError("candidate bin must contain actual Linux ELFs, not wrappers")
        shutil.copy2(member, output / "bin" / member.name)


def produce(args):
    output = Path(os.environ["out"])
    copy_binaries(args.candidate, output)
    executable = output / "bin/molten"
    shutil.copytree(args.gates / "primary", output / "primary", symlinks=True)
    shutil.copytree(args.gates / "library", output / "library", symlinks=True)
    artifacts = []
    for role, directory in [("primary-cohort", "primary"), ("library-cohort", "library")]:
        artifacts.append({"role": role, "content_ref": cohort_ref(output / directory)})
    for role, name in [("gate-receipt", "gate-receipt.preserves"),
                       ("remediation-receipt", "remediation-plan.preserves")]:
        shutil.copy2(args.gates / name, output / name)
        artifacts.append({"role": role, "content_ref": file_ref(output / name)})
    graphs = [("candidate-build", args.closure), ("candidate-compiler", args.compiler_closure),
              ("candidate-runtime", args.runtime_closure), ("gate-build", args.gate_closure),
              ("gate-compiler", args.gate_compiler_closure), ("gate-runtime", args.gate_runtime_closure)]
    measured_closure(graphs, output / "build-closure")
    shutil.copy2(args.derivation, output / "candidate.drv")
    shutil.copy2(args.policy, output / "policy")
    shutil.copy2(args.profile, output / "profile")
    binding = dict(schema="molten-node-source-binding-v1", producer_profile="molten-nix-source-binding-v1",
                   executable_ref=file_ref(executable), source_ref=nar_ref(args.source),
                   dependencies_ref=file_ref(output / "build-closure"), toolchain_ref=nar_ref(args.toolchain),
                   policy_ref=file_ref(output / "policy"), profile_ref=file_ref(output / "profile"),
                   artifacts=artifacts)
    (output / "binding.json").write_text(json.dumps(binding, sort_keys=True, separators=(",", ":")) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["candidate", "source", "gates", "toolchain", "closure", "compiler-closure", "runtime-closure",
                 "gate-closure", "gate-compiler-closure", "gate-runtime-closure", "derivation", "policy", "profile"]:
        parser.add_argument("--" + name, type=Path, required=True)
    produce(parser.parse_args())


if __name__ == "__main__":
    main()
