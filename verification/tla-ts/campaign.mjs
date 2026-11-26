import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DIVE_RELEASE, DIVE_SCHEMA, diveArgs, parseDive, parseDiveVersion, readDiveReport, verifyDiveArchive } from "./dive.mjs";

export const PUBLICATION_SHA256 = "cf962f00ab297186f919e32c8f5cf28f6e78f1e92703b9a3d8f02044a487c84a";
export const CHECKER_TIMEOUT_MS = 180_000;
export const CAMPAIGN_TIMEOUT_MS = 720_000;

function requireThat(condition, message) {
  if (!condition) throw new Error(message);
}

export function parseTlc(text, exitCode) {
  requireThat(/TLC2 Version 2\.19\b/.test(text), "TLC version drift: expected TLC 2.19 / TLA+ 1.7.4");
  const count = text.match(/([\d,]+) states generated, ([\d,]+) distinct states found, ([\d,]+) states left on queue\./);
  requireThat(count, "TLC did not report completed-search statistics");
  const violation = text.match(/Error: Invariant (\w+) is violated\./);
  const traceLength = [...text.matchAll(/^State \d+:/gm)].length;
  const success = text.includes("Model checking completed. No error has been found.");
  requireThat((success && exitCode === 0 && !violation) || (violation && exitCode === 12 && traceLength > 0),
    `TLC incomplete or unexpected termination (exit ${exitCode})`);
  const number = (value) => Number(value.replaceAll(",", ""));
  return {
    tool: "TLC 2.19 / TLA+ 1.7.4",
    status: violation ? "invariant-violation" : "success",
    generatedStates: number(count[1]),
    distinctStates: number(count[2]),
    queuedStates: number(count[3]),
    violation: violation ? { name: violation[1], traceLength } : null,
  };
}

// Each mutant removes one protocol guard (missing promise) or shrinks a quorum
// below the intersection bound; each must break order agreement. The
// four-replica instance is where recovery-quorum intersection is load-bearing
// (its superquorum is smaller than the replica set).
export const MODEL_RUNS = Object.freeze([
  { label: "baseline", config: "ConsensusFastPath.cfg", violation: null,
    bounds: { replicas: 3, commands: 2, views: 3 } },
  { label: "missing-promise", config: "ConsensusFastPathMissingPromise.cfg", violation: "CommittedOrderAgreement",
    bounds: { replicas: 3, commands: 2, views: 3 } },
  { label: "insufficient-quorum", config: "ConsensusFastPathInsufficientQuorum.cfg", violation: "CommittedOrderAgreement",
    bounds: { replicas: 3, commands: 2, views: 3 } },
  { label: "four-replica", config: "ConsensusFastPathFourReplica.cfg", violation: null,
    bounds: { replicas: 4, commands: 2, views: 2 } },
  { label: "four-replica-small-recovery-quorum", config: "ConsensusFastPathFourReplicaSmallQuorum.cfg",
    violation: "CommittedOrderAgreement", bounds: { replicas: 4, commands: 2, views: 2 } },
]);

// r[impl molten.consensus.fast_path_model.fault_corpus]
// r[impl molten.consensus.fast_path_model.validation]
// r[impl molten.consensus.fast_path_model.dive_parity]
export function validatePair(tlc, ts, dive, violation = null) {
  const mutant = violation !== null;
  const expected = mutant ? "invariant-violation" : "success";
  for (const result of [tlc, ts, dive]) {
    requireThat(result.status === expected, `${result.tool}: expected ${expected}, got ${result.status}`);
    requireThat(Number.isSafeInteger(result.generatedStates) && result.generatedStates > 0,
      `${result.tool}: missing generated-state count`);
    requireThat(Number.isSafeInteger(result.distinctStates) && result.distinctStates > 0,
      `${result.tool}: missing distinct-state count`);
    requireThat(Number.isSafeInteger(result.queuedStates) && result.queuedStates >= 0,
      `${result.tool}: missing queued-state count`);
    requireThat(!result.failure, `${result.tool}: checker evaluation failure`);
    if (!mutant) requireThat(result.queuedStates === 0, `${result.tool}: search incomplete`);
    if (mutant) {
      requireThat(result.violation?.name === violation, `${result.tool}: wrong mutant violation`);
      requireThat((result.violation.traceLength ?? result.violation.trace?.length ?? 0) > 1,
        `${result.tool}: missing causal counterexample`);
    }
  }
  // Full searches must enumerate the same graph. A mutant stops at its first
  // violation; successor order can change its visited prefix, not its verdict.
  const divergences = [ts, dive].filter((result) =>
    tlc.distinctStates !== result.distinctStates || tlc.generatedStates !== result.generatedStates);
  const countsAgree = divergences.length === 0;
  if (!mutant && !countsAgree) {
    const counts = (result) => `${result.tool} ${result.generatedStates} generated / ${result.distinctStates} distinct`;
    throw new Error(`semantic parity failed: exhaustive state counts differ: ${counts(tlc)} vs ${divergences.map(counts).join(", ")}`);
  }
  return { verdictsAgree: true, countsAgree, exhaustive: !mutant };
}

export function requireCheckerCapture(result, stem, elapsedMs, timeout) {
  if (result.error?.code === "ETIMEDOUT" || elapsedMs > timeout) {
    throw new Error(`${stem}: checker timeout (limit ${timeout} ms)`);
  }
  requireThat(!result.error && !result.signal && Number.isInteger(result.status),
    `${stem}: checker process failed: ${result.error?.message ?? result.signal ?? "missing exit status"}`);
}

function bindRetainedFiles(output, names, remaining) {
  if (!names.length) return [];
  const result = spawnSync("b3sum", ["--", ...names], {
    cwd: output, encoding: "utf8", timeout: Math.max(1, Math.floor(remaining)), killSignal: "SIGKILL",
  });
  requireThat(result.status === 0 && !result.error && !result.signal,
    `BLAKE3 retention failed: ${result.error?.message || result.stderr || result.signal || `exit ${result.status}`}`);
  const lines = result.stdout.trimEnd().split("\n");
  requireThat(lines.length === names.length, "BLAKE3 retention returned incomplete digests");
  return names.map((name, index) => {
    const match = lines[index].match(/^([0-9a-f]{64})  (.+)$/);
    requireThat(match && match[2] === name, `BLAKE3 retention did not bind ${name}`);
    return { name, blake3: match[1] };
  });
}

// r[impl molten.consensus.fast_path_model.evidence]
// r[impl molten.consensus.fast_path_model.nonclaims]
export function runCampaign(outputDirectory, extractedRoot, diveArchive, diveExecutable) {
  const here = dirname(fileURLToPath(import.meta.url));
  const modelDirectory = resolve(here, "../tla");
  const output = resolve(outputDirectory);
  // Refuse reuse: a prior successful summary must never mask this invocation.
  mkdirSync(output);
  const started = performance.now();
  const remaining = () => CAMPAIGN_TIMEOUT_MS - (performance.now() - started);
  const report = {
    schema: "molten.consensus-model-campaign.v1",
    evidenceKind: "finite-model-only",
    claimScope: "Agreement with TLC is not proof of Aspen's Rust implementation.",
    status: "failed",
    authoritativeChecker: "tlc",
    bounds: Object.fromEntries(MODEL_RUNS.map((run) => [run.label, run.bounds])),
    perCheckerLimitMs: CHECKER_TIMEOUT_MS,
    campaignLimitMs: CAMPAIGN_TIMEOUT_MS,
    runs: [],
  };
  const save = () => {
    report.elapsedMs = performance.now() - started;
    writeFileSync(join(output, "campaign.json"), `${JSON.stringify(report, null, 2)}\n`);
  };
  const retained = [];
  const retain = (name, data) => {
    writeFileSync(join(output, name), data);
    retained.push(name);
  };
  const runCaptured = (stem, executable, args) => {
    const timeout = Math.min(CHECKER_TIMEOUT_MS, Math.floor(remaining()));
    requireThat(timeout > 0, "campaign deadline exceeded");
    const runStarted = performance.now();
    const result = spawnSync(executable, args, { timeout, killSignal: "SIGKILL", maxBuffer: 64 * 1024 * 1024 });
    const elapsedMs = performance.now() - runStarted;
    retain(`${stem}.stdout`, result.stdout ?? "");
    retain(`${stem}.stderr`, result.stderr ?? "");
    return { result, elapsedMs, timeout };
  };
  let failure;
  try {
    const archive = readFileSync(join(here, "tla-ts-0.0.10.tgz"));
    requireThat(createHash("sha256").update(archive).digest("hex") === PUBLICATION_SHA256,
      "TLA-ts archive pin mismatch; use the reviewed import procedure, never patch the archive");
    report.tlaTsPublicationSha256 = PUBLICATION_SHA256;
    report.diveRelease = { ...DIVE_RELEASE, reportSchema: DIVE_SCHEMA };
    verifyDiveArchive(readFileSync(resolve(diveArchive)));
    const dive = resolve(diveExecutable);
    const probeArgs = ["--version"];
    const probe = runCaptured("dive-version", dive, probeArgs);
    report.diveVersionProbe = {
      executable: dive, args: probeArgs, elapsedMs: probe.elapsedMs,
      exitCode: probe.result.status, signal: probe.result.signal,
    };
    requireCheckerCapture(probe.result, "dive-version", probe.elapsedMs, probe.timeout);
    report.diveVersion = parseDiveVersion(probe.result.stdout.toString("utf8"), probe.result.status);
    for (const name of ["ConsensusFastPath.tla", ...MODEL_RUNS.map((run) => run.config)]) {
      copyFileSync(join(modelDirectory, name), join(output, name));
      retained.push(name);
    }
    const model = join(output, "ConsensusFastPath.tla");
    report.baselines = {};
    report.mutants = {};
    for (const { label, config, violation } of MODEL_RUNS) {
      const summaries = [];
      const configPath = join(output, config);
      const mutant = violation !== null;
      for (const tool of ["tlc", "tla-ts", "dive"]) {
        const stem = `${label}-${tool}`;
        const [executable, args] = tool === "tlc"
          ? ["tlc", ["-workers", "1", "-metadir", join(output, `${stem}-states`), "-config", configPath, model]]
          : tool === "tla-ts"
            ? [process.execPath, [join(here, "check.mjs"), model, configPath, resolve(extractedRoot)]]
            : [dive, diveArgs(model, configPath)];
        const { result, elapsedMs, timeout } = runCaptured(stem, executable, args);
        const observed = { label, tool, elapsedMs, exitCode: result.status, signal: result.signal, executable, args };
        report.runs.push(observed);
        save();
        requireCheckerCapture(result, stem, elapsedMs, timeout);
        let summary;
        if (tool === "dive") {
          const nativeReport = readDiveReport(result.stdout, result.status, result.stderr.toString("utf8"));
          retain(`${stem}.report.json`, result.stdout);
          summary = parseDive(nativeReport, result.status, mutant);
        } else {
          summary = tool === "tlc"
            ? parseTlc(`${result.stdout}\n${result.stderr}`, result.status)
            : JSON.parse(result.stdout);
        }
        if (tool === "tla-ts") {
          requireThat(result.status === 0 && summary.tool === "tla-ts@0.0.10", `${stem}: checker API failure`);
        }
        observed.result = summary;
        summaries.push(summary);
        retain(`${stem}.json`, `${JSON.stringify(summary, null, 2)}\n`);
        save();
      }
      const verdict = validatePair(...summaries, violation);
      if (mutant) report.mutants[label] = { violation, ...verdict };
      else report.baselines[label] = verdict;
    }
  } catch (error) {
    failure = error;
  }
  try {
    report.retained_files = bindRetainedFiles(output, retained, remaining());
  } catch (error) {
    if (failure) report.retentionError = error.message;
    else failure = error;
  }
  if (!failure && remaining() < 0) failure = new Error(`campaign exceeded agreed ${CAMPAIGN_TIMEOUT_MS / 1000} second bound`);
  if (failure) report.error = `${failure.name}: ${failure.message}`;
  else report.status = "passed";
  save();
  if (failure) throw failure;
  return report;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [output, root, diveArchive, diveExecutable] = process.argv.slice(2);
  if (!output || !root || !diveArchive || !diveExecutable) {
    throw new Error("usage: campaign.mjs NEW_OUTPUT_DIRECTORY EXTRACTED_NPM_ROOT DIVE_ARCHIVE DIVE_EXECUTABLE");
  }
  runCampaign(output, root, diveArchive, diveExecutable);
}
