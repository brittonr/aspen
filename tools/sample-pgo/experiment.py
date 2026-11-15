#!/usr/bin/env python3
"""Opt-in, fail-closed sample-PGO experiment; JSON is diagnostic, not admission."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import resource
import signal
import statistics
import subprocess
import sys
import time


class Blocked(RuntimeError):
    pass


class SemanticMismatch(RuntimeError):
    pass


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def byte_identity(path):
    # Raw bytes, deliberately not a canonical Preserves value identity.
    result = subprocess.run(["b3sum", str(path)], check=True, capture_output=True, text=True)
    return {"raw_blake3": result.stdout.split()[0], "sha256": sha(path), "bytes": Path(path).stat().st_size}


def limits():
    resource.setrlimit(resource.RLIMIT_FSIZE, (134217728, 134217728))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_AS, (8589934592, 8589934592))


def invoke(command, directory, label, timeout, expected_success=True):
    """Retain commands and failed runs; timeout kills only this process group."""
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    prefix = directory / label
    started = time.monotonic()
    env = dict(os.environ)
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_BOOTSTRAP", "LD_PRELOAD", "LD_LIBRARY_PATH"):
        env.pop(key, None)
    env.update({"LC_ALL": "C", "TZ": "UTC"})
    timed_out = False
    with prefix.with_suffix(".stdout").open("wb") as stdout, prefix.with_suffix(".stderr").open("wb") as stderr:
        process = subprocess.Popen(
            ["time", "-f", "%e %U %S %M", "-o", str(prefix.with_suffix(".usage")), "--", *map(str, command)],
            stdout=stdout, stderr=stderr, env=env, start_new_session=True, preexec_fn=limits,
        )
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            code = process.wait()
    row = {"command": list(map(str, command)), "returncode": code,
           "timeout": timed_out, "elapsed_seconds": time.monotonic() - started}
    usage = prefix.with_suffix(".usage")
    if usage.exists() and usage.read_text().splitlines():
        fields = usage.read_text().splitlines()[-1].split()
        if len(fields) == 4:
            try:
                row.update(zip(("time_wall_seconds", "user_seconds", "system_seconds", "max_rss_kib"), map(float, fields)))
            except ValueError:
                pass
    save(prefix.with_suffix(".json"), row)
    if timed_out or (expected_success is not None and (code == 0) != expected_success):
        raise Blocked(f"{label}: unexpected exit {code}, timeout={timed_out}; see {prefix}.stderr")
    return row, prefix.with_suffix(".stdout").read_text()


# r[impl molten.sample_pgo.replay]
def suite(label, seed, steps, budget=64, grants=True):
    capabilities = '<grant "producer" "assert" #f #f> <grant "producer" "send" "sink" #f> <grant "producer" "clock" #f #f>' if grants else ""
    return f'''<harness-suite-v1 "molten.harness.suite.v1" "{label}" {seed}
  <budget-v1 "molten.harness.budget.v1" <limits {budget} 64 4096 4194304>>
  <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native"> <actor "sink" "native">]>
  <capabilities-v1 "molten.harness.capabilities.v1" [{capabilities}]>
  [{" ".join(steps)}]>
'''


def datasets(policy, directory):
    """Materialize exact source-owned native harness inputs, never result fixtures."""
    directory = Path(directory)
    directory.mkdir()
    train = ['<clock "producer">' if i % 6 == 5 else
             f'<assert "producer" <training-payload {i}>>' if i % 2 == 0 else
             f'<send "producer" "sink" <training-payload {i}>>' for i in range(policy["training_steps"])]
    held = ['<clock "producer">' if i % 6 == 5 else
            f'<assert "producer" <held-out-payload {i} "evaluation-only">>' if i % 2 == 0 else
            f'<send "producer" "sink" <held-out-payload {i} "evaluation-only">>' for i in range(policy["evaluation_steps"])]
    sources = {
        "training": suite("sample-pgo-training", policy["training_seed"], train),
        "success": suite("sample-pgo-held-out", policy["evaluation_seed"], held),
        "denied": suite("sample-pgo-held-out-denied", policy["evaluation_seed"], ['<clock "producer">'], grants=False),
        "exhausted": suite("sample-pgo-held-out-exhausted", policy["evaluation_seed"], ['<clock "producer">'] * 5, budget=4),
        "error": suite("sample-pgo-held-out-error", policy["evaluation_seed"], ['<clock "unknown-actor">']),
    }
    paths = {}
    for name, text in sources.items():
        paths[name] = directory / f"{name}.preserves"
        paths[name].write_text(text)
    return paths


def run_case(binary, case, path, directory, timeout):
    directory = Path(directory)
    directory.mkdir(parents=True)
    artifact = directory / "artifact.preserves"
    row, _ = invoke([binary, "test", "run", path, "--report-out", artifact], directory, "run", timeout,
                    expected_success=case in ("training", "success", "denied"))
    if case in ("exhausted", "error") and row["returncode"] != 1:
        raise Blocked(f"{case}: expected CLI error exit 1, not a signal or another failure class")
    if not artifact.is_file() or artifact.stat().st_size == 0:
        raise Blocked(f"{case}: missing actual canonical harness artifact")
    # Additional outcome assertions, never substitutes for the owner replay gate.
    text = artifact.read_text()
    if case == "denied" and '"deny"' not in text:
        raise SemanticMismatch("denied workload did not record denial")
    if case == "exhausted" and "suite step count exceeds budget" not in text:
        raise SemanticMismatch("exhausted workload did not record its resource failure")
    if case == "error" and "unknown actor" not in text:
        raise SemanticMismatch("error workload did not record its actor error")
    row["artifact"] = byte_identity(artifact)
    return row, artifact


def equivalence(baseline, candidate, paths, directory, timeout):
    rows = {}
    for case in ("success", "denied", "exhausted", "error"):
        left, expected = run_case(baseline, case, paths[case], directory / case / "baseline", timeout)
        right, actual = run_case(candidate, case, paths[case], directory / case / "candidate", timeout)
        if expected.read_bytes() != actual.read_bytes():
            raise SemanticMismatch(f"{case}: canonical harness artifacts diverged")
        if left["returncode"] != right["returncode"]:
            raise SemanticMismatch(f"{case}: CLI exit classes diverged")
        for variant, binary, artifact in (("baseline", baseline, expected), ("candidate", candidate, actual)):
            invoke([binary, "test", "report", "show", artifact], directory / case,
                   f"canonical-readback-{variant}", timeout)
        if case in ("success", "denied"):
            # Cross replay each report in both variants with the embedded effect
            # log; the owner validates canonical observations/receipts/state refs.
            for variant, binary in (("baseline", baseline), ("candidate", candidate)):
                for origin, report in (("baseline", expected), ("candidate", actual)):
                    invoke([binary, "test", "replay", report], directory / case,
                           f"replay-{variant}-{origin}", timeout)
        rows[case] = {"baseline": left, "candidate": right,
                      "canonical_equal": True, "replay": "passed" if case in ("success", "denied") else "not-applicable-preflight-failure"}
    save(directory / "equivalence.json", rows)
    return rows


# r[impl molten.sample_pgo.measurement]
# r[impl molten.sample_pgo.non_claim]
def summarize(pairs, binary_sizes, policy):
    improvements = []
    case_rows = {}
    rss_ratios = []
    for case in ("success", "denied", "exhausted", "error"):
        observations = [pair[case] for pair in pairs]
        base = [row["baseline"]["elapsed_seconds"] for row in observations]
        candidate = [row["candidate"]["elapsed_seconds"] for row in observations]
        case_rows[case] = {"baseline_median_seconds": statistics.median(base),
                           "candidate_median_seconds": statistics.median(candidate),
                           "median_regression": statistics.median(candidate) / statistics.median(base) - 1,
                           "baseline_range_seconds": [min(base), max(base)],
                           "candidate_range_seconds": [min(candidate), max(candidate)]}
        for variant in ("baseline", "candidate"):
            cpu = [row[variant]["user_seconds"] + row[variant]["system_seconds"] for row in observations]
            memory = [row[variant]["max_rss_kib"] for row in observations]
            case_rows[case][f"{variant}_median_cpu_seconds"] = statistics.median(cpu)
            case_rows[case][f"{variant}_peak_rss_kib"] = max(memory)
            case_rows[case][f"{variant}_requests_per_second"] = 1 / statistics.median(
                [row[variant]["elapsed_seconds"] for row in observations])
        for row in observations:
            rss_ratios.append(row["candidate"]["max_rss_kib"] / max(row["baseline"]["max_rss_kib"], 1))
    for pair in pairs:
        base = sum(pair[case]["baseline"]["elapsed_seconds"] for case in pair)
        candidate = sum(pair[case]["candidate"]["elapsed_seconds"] for case in pair)
        improvements.append(1 - candidate / base)
    mean = statistics.mean(improvements)
    margin = policy["confidence_multiplier"] * statistics.stdev(improvements) / math.sqrt(len(improvements))
    lower, upper = mean - margin, mean + margin
    regression = max(row["median_regression"] for row in case_rows.values())
    rss_growth = max(rss_ratios) - 1
    binary_growth = binary_sizes["candidate"] / binary_sizes["baseline"] - 1
    if regression > policy["maximum_case_median_regression"] or rss_growth > policy["maximum_peak_rss_growth"] or binary_growth > policy["maximum_binary_growth"]:
        decision = "revise"
    elif lower > policy["minimum_wall_improvement"]:
        decision = "keep"
    elif upper < policy["minimum_wall_improvement"]:
        decision = "stop"
    else:
        decision = "inconclusive"
    return {"decision": decision, "scope": "machine-local CLI workload only; not release admission",
            "mean_paired_wall_improvement": mean, "approximate_95_percent_interval": [lower, upper],
            "interval_method": "paired mean Student-t interval, df=29, predeclared multiplier=2.04523; assumes independent pairs",
            "cases": case_rows, "maximum_paired_peak_rss_growth": rss_growth, "binary_growth": binary_growth,
            "retained_memory": "unsupported: each client process exits; peak RSS is measured, retained heap is not",
            "p99": f"unsupported: fewer than {policy['minimum_tail_samples']} observations per path",
            "throughput": "inverse complete CLI request wall latency; raw per-path samples retained",
            "build_cpu_and_peak_memory": "unsupported for daemon builders; build records measure Nix client and realization wall only"}


def host_snapshot():
    paths = ["/proc/cpuinfo", "/proc/meminfo", "/proc/loadavg", "/proc/sys/kernel/perf_event_paranoid",
             "/proc/sys/kernel/kptr_restrict", "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
             "/sys/fs/cgroup/cpu.max", "/sys/fs/cgroup/memory.max", "/proc/self/status"]
    return {"platform": platform.platform(), "affinity": sorted(os.sched_getaffinity(0)),
            "conditions": {name: Path(name).read_text() if Path(name).is_file() else "unavailable" for name in paths}}


def build(expression, out, label, timeout):
    row, stdout = invoke(["nix", "build", "--impure", "--out-link", out / f"{label}-result", "--print-out-paths", "--expr", expression], out, label, timeout)
    lines = stdout.splitlines()
    if len(lines) != 1 or not lines[0].startswith("/nix/store/"):
        raise Blocked(f"{label}: expected one immutable Nix output")
    store = Path(lines[0])
    if not (store / "bin/molten").is_file():
        raise Blocked(f"{label}: Nix output lacks the real Aspen CLI binary (bin/molten)")
    row["store_path"] = str(store)
    row["derivation_scope"] = "realization wall time and Nix client resources; may use substituters/cache"
    save(out / f"{label}-realization.json", row)
    return store


def profile_samples(text):
    # LLVM sample text headers are name:total:head (body lines are indented).
    counts = [int(match[1]) for line in text.splitlines()
              if (match := re.match(r"^\S*molten\S*:([0-9]+):[0-9]+$", line))]
    return sum(counts)


def build_id_associations(text):
    # perf's no-ID rows have a blank ID column. Only its freshly materialized
    # vDSO name is unstable; preserve all other paths, even without an ID.
    associations = {}
    for line in text.splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r"([a-f0-9]{2}(?:[a-f0-9]{2})*)\s+(/.+)", line)
        if match:
            build_id, path = match.groups()
        elif line[0].isspace() and line.strip().startswith("/"):
            build_id, path = "", line.strip()
            if re.fullmatch(r"/tmp/perf-vdso\.so-[A-Za-z0-9]+", path):
                continue
        else:
            raise Blocked("malformed perf build-ID association row")
        if path in associations and associations[path] != build_id:
            raise Blocked("conflicting perf build IDs for one path")
        associations[path] = build_id
    return sorted((build_id, path) for path, build_id in associations.items())


# r[impl molten.sample_pgo.capture]
def validate_profile(bundle, binary, converter, directory, minimum_samples):
    """Re-derive the sample in the Nix builder; caller assertions are not proof."""
    bundle, binary, directory = Path(bundle), Path(binary), Path(directory)
    directory.mkdir(parents=True, exist_ok=False)
    binding = json.loads((bundle / "binding.json").read_text())
    for key, path in (("baseline_sha256", binary), ("raw_sha256", bundle / "perf.data"),
                      ("association_sha256", bundle / "association.txt"), ("profile_sha256", bundle / "sample.prof"),
                      ("converter_sha256", converter)):
        if sha(path) != binding[key]:
            raise Blocked(f"stale {key}")
    _, elf = invoke(["readelf", "-n", binary], directory, "elf", 30)
    match = re.search(r"Build ID: ([a-f0-9]+)", elf)
    if not match or binding["baseline_elf_build_id"] != match[1]:
        raise Blocked("stale baseline ELF build ID")
    # Store imports may have another build user's UID. Parse an owned copy under
    # the same file-size/time limits rather than bypassing perf's ownership check.
    raw = directory / "perf.data"
    invoke(["cp", "--", bundle / "perf.data", raw], directory, "raw-copy", 30)
    if sha(raw) != binding["raw_sha256"]:
        raise Blocked("stale copied raw_sha256")
    _, association = invoke(["perf", "buildid-list", "--with-hits", "-i", raw], directory, "association", 30)
    associations = build_id_associations(association)
    if associations != build_id_associations((bundle / "association.txt").read_text()) or (
            match[1], str(binary)) not in associations:
        raise Blocked("raw sample association differs from baseline")
    regenerated = directory / "regenerated.prof"
    invoke([converter, f"--binary={binary}", f"--perfdata={raw}",
            f"--output={regenerated}", "--format=text"], directory, "convert", 180)
    samples = profile_samples(regenerated.read_text())
    if samples < minimum_samples or samples != binding["application_samples"]:
        raise Blocked("empty or mismatched application sample counts")
    if regenerated.read_bytes() != (bundle / "sample.prof").read_bytes():
        raise Blocked("converted profile differs from the pinned converter output")


# r[impl molten.sample_pgo.cohort]
# r[impl molten.sample_pgo.capture]
def experiment(cohort, policy, out):
    save(out / "cohort.json", cohort)
    save(out / "policy.json", policy)
    save(out / "host-before.json", host_snapshot())
    if platform.machine() != "x86_64" or platform.system() != "Linux":
        raise Blocked("experiment supports only x86_64-linux")
    if policy["sampling_max_bytes"] != 134217728:
        raise Blocked("declared sampling byte bound differs from the enforced process file-size cap")
    _, version = invoke([cohort["compiler"], "-vV"], out, "compiler", 30)
    if cohort["compiler_revision"] not in version or "1.100.0-nightly" not in version:
        raise Blocked("compiler version/commit differs from selected experimental cohort")
    invoke([cohort["converter"], "--version"], out, "converter", 30)
    invoke(["perf", "version"], out, "perf-version", 30)
    invoke(["nix", "--version"], out, "nix-version", 30)
    paths = datasets(policy, out / "datasets")
    save(out / "dataset-identities.json", {name: byte_identity(path) for name, path in paths.items()})
    baseline = build(cohort["baseline_expression"], out, "baseline-build", policy["build_timeout_seconds"])
    binary = baseline / "bin/molten"
    invoke(["readelf", "-n", binary], out, "baseline-elf", 30)
    elf = (out / "baseline-elf.stdout").read_text()
    match = re.search(r"Build ID: ([a-f0-9]+)", elf)
    if not match:
        raise Blocked("baseline lacks an ELF build ID")
    bundle = out / "profile-bundle"
    bundle.mkdir()
    # No event fallback, global attach, privilege escalation, or .fxt import.
    # Keep recorded build IDs, but do not copy large ELFs into perf's user cache
    # under the inherited per-file cap; conversion uses the exact store binary.
    invoke(["perf", "record", "--no-buildid-cache", "-o", bundle / "perf.data", "-e", policy["sampling_event"], "-b", "-F", str(policy["sampling_frequency"]),
            "--", sys.executable, __file__, "train", "--binary", binary, "--suite", paths["training"],
            "--out", out / "training", "--seconds", str(policy["sampling_seconds"])], out, "capture", policy["sampling_timeout_seconds"])
    _, association = invoke(["perf", "buildid-list", "--with-hits", "-i", bundle / "perf.data"], out, "association", 30)
    if not any(line.split() == [match[1], str(binary)] for line in association.splitlines()):
        raise Blocked("perf hit build-ID association does not match baseline binary")
    (bundle / "association.txt").write_text(association)
    _, branches = invoke(["perf", "script", "-i", bundle / "perf.data", "-F", "ip,brstack"], out, "branches", 30)
    if not re.search(r"(?:0x)?[a-fA-F0-9]+/(?:0x)?[a-fA-F0-9]+/", branches):
        raise Blocked("perf recorded no usable hardware branch stack")
    invoke([cohort["converter"], f"--binary={binary}", f"--perfdata={bundle / 'perf.data'}",
            f"--output={bundle / 'sample.prof'}", "--format=text"], out, "convert", 180)
    samples = profile_samples((bundle / "sample.prof").read_text())
    if samples < policy["minimum_application_samples"]:
        raise Blocked(f"too few actual Aspen application profile samples: {samples}")
    source = Path(cohort["source"])
    binding = {"schema": "molten.sample-pgo.profile-binding.v1", "source": str(source),
               "compiler_revision": cohort["compiler_revision"], "llvm_revision": cohort["llvm_revision"],
               "flags": cohort["flags"], "baseline": str(baseline), "baseline_sha256": sha(binary),
               "compiler_sha256": sha(cohort["compiler"]), "converter_sha256": sha(cohort["converter"]),
               "lock_sha256": sha(source / "Cargo.lock"), "plan_sha256": sha(source / "build-plan.json"),
               "policy_sha256": sha(cohort["policy"]), "raw_sha256": sha(bundle / "perf.data"),
               "association_sha256": sha(bundle / "association.txt"), "profile_sha256": sha(bundle / "sample.prof"),
               "application_samples": samples, "baseline_elf_build_id": match[1]}
    save(bundle / "binding.json", binding)
    _, immutable = invoke(["nix", "store", "add-path", bundle], out, "profile-import", 30)
    immutable = immutable.strip()
    if not re.fullmatch(r"/nix/store/[a-z0-9]{32}-[a-zA-Z0-9+._?=-]+", immutable):
        raise Blocked("profile import did not yield one immutable store path")
    candidate = build(f"{cohort['candidate_expression']} {immutable}", out, "candidate-build", policy["build_timeout_seconds"])
    candidate_binary = candidate / "bin/molten"
    semantic = equivalence(binary, candidate_binary, paths, out / "semantics", policy["invocation_timeout_seconds"])
    pairs = []
    for repetition in range(-policy["warmups"], policy["repetitions"]):
        pair = {}
        order = [("baseline", binary), ("candidate", candidate_binary)]
        if repetition % 2:
            order.reverse()
        for case in ("success", "denied", "exhausted", "error"):
            pair[case] = {}
            for variant, executable in order:
                row, artifact = run_case(executable, case, paths[case], out / "measurements" / str(repetition) / case / variant,
                                         policy["invocation_timeout_seconds"])
                if "max_rss_kib" not in row:
                    raise Blocked("missing raw CPU/peak-RSS measurement")
                if row["artifact"] != semantic[case][variant]["artifact"]:
                    raise SemanticMismatch(f"{case}: measured invocation canonical artifact drifted")
                pair[case][variant] = row
        if repetition >= 0:
            pairs.append(pair)
            save(out / "raw-pairs.json", pairs)
    sizes = {"baseline": binary.stat().st_size, "candidate": candidate_binary.stat().st_size}
    save(out / "binary-identities.json", {"baseline": byte_identity(binary), "candidate": byte_identity(candidate_binary)})
    result = summarize(pairs, sizes, policy)
    result.update({"status": "measured", "canonical_equivalence": "passed", "profile_bundle": immutable,
                   "maintained_profile": "not-enabled; a separate reviewed Cairn change is required even for keep"})
    save(out / "host-after.json", host_snapshot())
    save(out / "result.json", result)


def train(args):
    directory = Path(args.out)
    directory.mkdir()
    deadline = time.monotonic() + args.seconds
    count = 0
    while time.monotonic() < deadline:
        # Bound every inherited child; raw training runs are retained as well.
        run_case(Path(args.binary), "training", Path(args.suite), directory / str(count), 30)
        count += 1
    if count == 0:
        raise Blocked("training completed no real workload invocation")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("run")
    run.add_argument("--cohort", required=True)
    run.add_argument("--out", required=True)
    training = commands.add_parser("train")
    training.add_argument("--binary", required=True)
    training.add_argument("--suite", required=True)
    training.add_argument("--out", required=True)
    training.add_argument("--seconds", type=float, required=True)
    validation = commands.add_parser("validate-profile")
    validation.add_argument("--bundle", required=True)
    validation.add_argument("--binary", required=True)
    validation.add_argument("--converter", required=True)
    validation.add_argument("--out", required=True)
    validation.add_argument("--minimum-samples", type=int, required=True)
    args = parser.parse_args()
    if args.command == "train":
        train(args)
        return
    if args.command == "validate-profile":
        validate_profile(args.bundle, args.binary, args.converter, args.out, args.minimum_samples)
        return
    out = Path(args.out).absolute()
    out.mkdir(parents=True, exist_ok=False)
    try:
        cohort = json.loads(Path(args.cohort).read_text())
        policy = json.loads(Path(cohort["policy"]).read_text())
        experiment(cohort, policy, out)
    except (Blocked, SemanticMismatch, OSError, ValueError, subprocess.SubprocessError) as error:
        save(out / "result.json", {"status": "ineligible" if isinstance(error, SemanticMismatch) else "blocked",
                                   "decision": None, "reason": str(error), "role": "development-observation"})
        save(out / "host-after.json", host_snapshot())
        print(error, file=sys.stderr)
        raise SystemExit(1) from error


if __name__ == "__main__":
    main()
