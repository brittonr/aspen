import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

export const DIVE_RELEASE = Object.freeze(JSON.parse(readFileSync(new URL("./dive-release.json", import.meta.url))));
export const DIVE_SCHEMA = 3;

function requireThat(condition, message) {
  if (!condition) throw new Error(`Dive ${message}`);
}

export function verifyDiveArchive(bytes) {
  requireThat(createHash("sha256").update(bytes).digest("hex") === DIVE_RELEASE.sha256, "archive pin mismatch");
}

export function parseDiveVersion(stdout, exitCode) {
  const version = stdout.trim();
  requireThat(exitCode === 0, `version probe failed (exit ${exitCode})`);
  requireThat(version === DIVE_RELEASE.version, `version drift: expected ${DIVE_RELEASE.version}, got ${version}`);
  return version;
}

export function diveArgs(model, config) {
  return ["check", model, "--config", config, "--workers", "1", "--compat=tlc", "--output", "json"];
}

export function readDiveReport(stdout, exitCode, stderr = "") {
  try {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(stdout));
  } catch (error) {
    // Some startup refusals emit their named cause only on stderr.
    const cause = exitCode === 3 ? "refused the check" : exitCode === 5 ? "budget stop" : "report rejected";
    throw new Error(`Dive ${cause} (exit ${exitCode}): malformed or incomplete JSON report: ${stderr.trim() || error.message}`);
  }
}

// This admits only the two report families used by this campaign, not an arbitrary Dive check.
// See docs/user/reports-and-errors.md and crates/dive-api/src/check.rs at the release commit.
// r[impl molten.consensus.fast_path_model.dive_parity]
export function parseDive(report, exitCode, mutant = false) {
  requireThat(report?.schema_version === DIVE_SCHEMA && report.command === "check",
    `unsupported report schema or command: expected schema ${DIVE_SCHEMA} check`);
  requireThat(Number.isInteger(exitCode) && report.exit_code === exitCode,
    `incomplete report or exit code mismatch (process ${exitCode}, report ${report.exit_code})`);
  const { verdict, message, statistics, counterexample, property, options } = report;
  if (exitCode === 3 || verdict === "unsupported") {
    throw new Error(`Dive refused the check (exit ${exitCode}): ${message ?? verdict}`);
  }
  if (exitCode === 5 || verdict === "stopped" || report.stopped != null) {
    throw new Error(`Dive budget stop (exit ${exitCode}): ${message ?? verdict}`);
  }
  const expected = mutant ? "invariant-violation" : "success";
  requireThat(verdict === expected && exitCode === (mutant ? 12 : 0),
    `expected ${expected}, got ${verdict} (exit ${exitCode})${message ? `: ${message}` : ""}`);
  requireThat(options?.semantic_profile === "tlc" && options.exit_codes === "tlc" &&
    options.workers?.requested === 1 && options.workers.used === 1, "report did not establish single-worker TLC compatibility");

  const { generated, distinct, left_on_queue: queued } = statistics ?? {};
  requireThat(Number.isSafeInteger(generated) && Number.isSafeInteger(distinct) && Number.isSafeInteger(queued) &&
    distinct > 0 && generated >= distinct && queued >= 0 && queued <= distinct,
    "missing or invalid generated/distinct/queued state counts");
  let violation = null;
  if (mutant) {
    requireThat(report.check_kind === "safety" && typeof property === "string" && property.length > 0,
      "missing named safety violation");
    const { states, ending, length } = counterexample ?? {};
    requireThat(ending === "finite" && Array.isArray(states) && length === states.length && length > 1 &&
      states.every((state, index) => state?.index === index + 1 && state.values &&
        typeof state.values === "object" && !Array.isArray(state.values) && Object.keys(state.values).length > 0),
      "missing or incomplete causal counterexample");
    violation = { name: property, traceLength: length };
  } else {
    const { capture_completed: captured, exhausted, stage } = report.execution ?? {};
    requireThat(report.check_kind === "temporal" && captured === true && exhausted === true && stage === "complete" &&
      statistics.exhaustive !== false && queued === 0 && report.checkpoint?.status !== "paused-incomplete",
      "incomplete temporal search");
    requireThat(property === null && counterexample === null, "unexpected violation data on success");
  }
  return {
    tool: DIVE_RELEASE.version,
    status: verdict,
    generatedStates: generated,
    distinctStates: distinct,
    queuedStates: queued,
    violation,
  };
}
