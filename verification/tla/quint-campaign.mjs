import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Binary arguments are the normal and actual patch-mutated Cargo test binaries,
// built by the parent from the exact source cohort. No substitute Rust core.
const [outputArgument, normalArgument, mutantArgument] = process.argv.slice(2);
if (!outputArgument || !normalArgument || !mutantArgument) {
  throw new Error("usage: quint-campaign.mjs NEW_OUTPUT NORMAL_FASTPATHQUINT_BINARY MUTATED_FASTPATHQUINT_BINARY");
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const out = resolve(outputArgument);
const normal = resolve(normalArgument);
const mutant = resolve(mutantArgument);
if (normal === mutant) throw new Error("normal and mutant binaries must be separate build outputs");
mkdirSync(out);
const report = { schema: "molten.fastpath-quint-conformance.v1", status: "failed", runs: [] };
const started = performance.now();
function save() {
  report.elapsedMs = performance.now() - started;
  writeFileSync(join(out, "campaign.json"), `${JSON.stringify(report, null, 2)}\n`);
}
function run(label, executable, args, expectedFailure = false) {
  const before = performance.now();
  const result = spawnSync(executable, args, { cwd: root, encoding: "utf8", timeout: 180_000,
    maxBuffer: 64 * 1024 * 1024, killSignal: "SIGKILL", env: { ...process.env, QUINT_VERBOSE: "2" } });
  writeFileSync(join(out, `${label}.stdout`), result.stdout ?? "");
  writeFileSync(join(out, `${label}.stderr`), result.stderr ?? "");
  report.runs.push({ label, executable, args, elapsedMs: performance.now() - before,
    exitCode: result.status, error: result.error?.message ?? null, signal: result.signal });
  save();
  if (result.error || result.signal) throw new Error(`${label}: incomplete execution`);
  if (!expectedFailure && result.status !== 0) throw new Error(`${label}: nonzero exit ${result.status}`);
  if (expectedFailure && (result.status === 0 || !`${result.stdout}\n${result.stderr}`.includes("State invariant failed"))) {
    throw new Error(`${label}: mutation was not caught by state comparison`);
  }
  return `${result.stdout}\n${result.stderr}`;
}
try {
  const version = run("version", "quint", ["--version"]).trim();
  if (version.split(/\s+/).at(-1) !== "0.33.1") throw new Error("Quint version drift");
  const fixturePath = join(root, "verification/tla/fixtures/mixed-view-guard.json");
  const fixture = JSON.parse(readFileSync(fixturePath, "utf8"));
  copyFileSync(fixturePath, join(out, "mixed-view-guard.json"));
  copyFileSync(join(root, fixture.mutation), join(out, "mixed-view-guard.patch"));
  copyFileSync(join(root, fixture.spec), join(out, "ConsensusFastPath.qnt"));
  run("typecheck", "quint", ["typecheck", fixture.spec]);
  // The pinned Nix wrapper selects TypeScript, including quint-connect's calls.
  run("bounded-properties", "quint", ["run", fixture.spec, "--invariant", "inv", "--seed", "5",
    "--max-samples", "64", "--max-steps", "48", "--n-traces", "64",
    "--out-itf", join(out, "simulation-{seq}.itf.json")]);
  run("minimal-trace", "quint", ["test", fixture.spec, "--match", `^${fixture.run}$`, "--seed", "5",
    "--max-samples", "1", "--out-itf", join(out, "minimal-{seq}.itf.json")]);
  const traces = readdirSync(out).filter((name) => name.startsWith("minimal-") && name.endsWith(".itf.json"));
  if (traces.length !== 1) throw new Error("expected one generated minimal ITF trace");
  const trace = JSON.parse(readFileSync(join(out, traces[0]), "utf8"));
  if (trace.states?.length !== fixture.states || trace.states.at(-1).state?.[fixture.projection] !== fixture.expected) {
    throw new Error("generated causal trace does not match the authored two-state fixture");
  }
  const replay = run("normal-replay", normal, ["--nocapture", "--test-threads=1"]);
  if (!replay.includes("test result: ok. 5 passed; 0 failed")) throw new Error("normal replay test inventory mismatch");
  run("mutated-guard-replay", mutant, ["--exact", "mixed_view_guard_replay", "--nocapture"], true);
  report.status = "passed";
  report.scope = "bounded API conformance, not TLA+/Quint equivalence or live-engine proof";
  save();
} catch (error) {
  report.error = `${error.name}: ${error.message}`;
  save();
  throw error;
}
