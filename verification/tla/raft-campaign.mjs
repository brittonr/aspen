import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseTlc, requireCheckerCapture } from "../tla-ts/campaign.mjs";

// TLC is the single authoritative checker for the production Raft model.
// Every mutant removes one guard of the Rust transition function and must be
// rejected with its named invariant and a causal multi-state trace at the same
// bounds as an exhaustive passing baseline.
const RAFT = "AspenRaft.tla";
export const RUNS = Object.freeze([
  { label: "baseline-elections", model: RAFT, config: "AspenRaft.cfg", expected: null },
  { label: "commit-previous-term", model: RAFT, config: "AspenRaftCommitPreviousTerm.cfg", expected: "LeaderCompleteness" },
  { label: "vote-without-up-to-date", model: RAFT, config: "AspenRaftVoteWithoutUpToDate.cfg", expected: "LeaderCompleteness" },
  { label: "baseline-replication", model: RAFT, config: "AspenRaftReplication.cfg", expected: null },
  { label: "append-without-prev-match", model: RAFT, config: "AspenRaftAppendWithoutPrevMatch.cfg", expected: "LogMatching" },
  { label: "baseline-reads", model: RAFT, config: "AspenRaftReads.cfg", expected: null },
  { label: "read-without-current-commit", model: RAFT, config: "AspenRaftReadWithoutCurrentCommit.cfg", expected: "ReadIndexLinearizable" },
  { label: "minority-quorum", model: RAFT, config: "AspenRaftMinorityQuorum.cfg", expected: "ElectionSafety" },
  { label: "baseline-reordering", model: RAFT, config: "AspenRaftReordering.cfg", expected: null },
  // The live-trace observation window; its Init precondition is the
  // LeaderCompleteness invariant checked above.
  { label: "read-index-window", model: "RaftReadIndexWindow.tla", config: "RaftReadIndexWindow.cfg", expected: null },
]);
export const RUN_TIMEOUT_MS = 2_400_000;

function requireThat(condition, message) {
  if (!condition) throw new Error(message);
}

// r[verify molten.consensus.production_raft_formal_model]
export function validateRun(summary, expected) {
  requireThat(Number.isSafeInteger(summary.distinctStates) && summary.distinctStates > 0,
    `${summary.tool}: missing distinct-state count`);
  if (expected === null) {
    requireThat(summary.status === "success", `baseline: expected success, got ${summary.status}`);
    requireThat(summary.queuedStates === 0, "baseline: search incomplete");
    return { exhaustive: true };
  }
  requireThat(summary.status === "invariant-violation", `mutant: expected invariant-violation, got ${summary.status}`);
  requireThat(summary.violation?.name === expected,
    `mutant: expected ${expected}, got ${summary.violation?.name ?? "no violation"}`);
  requireThat(summary.violation.traceLength > 1, "mutant: missing causal counterexample");
  return { exhaustive: false };
}

export function runCampaign(outputDirectory, workers) {
  const modelDirectory = dirname(fileURLToPath(import.meta.url));
  const output = resolve(outputDirectory);
  // Refuse reuse: a prior successful summary must never mask this invocation.
  mkdirSync(output);
  const started = performance.now();
  const report = {
    schema: "molten.production-raft-model-campaign.v1",
    evidenceKind: "finite-model-only",
    claimScope: "Bounded TLC checking of a design model; not a proof of Aspen's Rust implementation.",
    status: "failed",
    checker: "TLC 2.19 / TLA+ 1.7.4",
    runs: [],
  };
  const save = () => {
    report.elapsedMs = performance.now() - started;
    writeFileSync(join(output, "campaign.json"), `${JSON.stringify(report, null, 2)}\n`);
  };
  try {
    for (const name of new Set(RUNS.flatMap((run) => [run.model, run.config]))) {
      copyFileSync(join(modelDirectory, name), join(output, name));
    }
    for (const { label, model, config, expected } of RUNS) {
      // Mutants use one worker so the first reported violation is deterministic.
      // TLC's state store is scratch, never evidence.
      const states = mkdtempSync(join(tmpdir(), `aspen-raft-${label}-`));
      const args = ["-workers", expected === null ? String(workers) : "1", "-metadir", states,
        "-config", join(output, config), join(output, model)];
      const runStarted = performance.now();
      const result = spawnSync("tlc", args, { timeout: RUN_TIMEOUT_MS, killSignal: "SIGKILL", maxBuffer: 256 * 1024 * 1024 });
      const elapsedMs = performance.now() - runStarted;
      writeFileSync(join(output, `${label}.stdout`), result.stdout ?? "");
      writeFileSync(join(output, `${label}.stderr`), result.stderr ?? "");
      const observed = { label, config, expected, args, elapsedMs, exitCode: result.status, signal: result.signal };
      report.runs.push(observed);
      save();
      requireCheckerCapture(result, label, elapsedMs, RUN_TIMEOUT_MS);
      observed.result = parseTlc(`${result.stdout}\n${result.stderr}`, result.status);
      observed.validation = validateRun(observed.result, expected);
      save();
    }
    report.status = "passed";
  } catch (error) {
    report.error = `${error.name}: ${error.message}`;
    save();
    throw error;
  }
  save();
  return report;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [output, workers] = process.argv.slice(2);
  if (!output || !/^[1-9][0-9]*$/.test(workers ?? "")) {
    throw new Error("usage: raft-campaign.mjs NEW_OUTPUT_DIRECTORY BASELINE_WORKERS");
  }
  runCampaign(output, Number(workers));
}
