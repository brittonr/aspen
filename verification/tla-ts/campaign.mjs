import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

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

// r[impl molten.consensus.fast_path_model.fault_corpus]
// r[impl molten.consensus.fast_path_model.validation]
export function validatePair(tlc, ts, mutant = false) {
  const expected = mutant ? "invariant-violation" : "success";
  for (const result of [tlc, ts]) {
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
      requireThat(result.violation?.name === "NoMissingPromise", `${result.tool}: wrong mutant violation`);
      requireThat((result.violation.traceLength ?? result.violation.trace?.length ?? 0) > 1,
        `${result.tool}: missing causal counterexample`);
    }
  }
  // Full searches must enumerate the same graph. A mutant stops at its first
  // violation; successor order can change its visited prefix, not its verdict.
  const countsAgree = tlc.distinctStates === ts.distinctStates && tlc.generatedStates === ts.generatedStates;
  if (!mutant) requireThat(countsAgree, "semantic parity failed: exhaustive state counts differ");
  return { verdictsAgree: true, countsAgree, exhaustive: !mutant };
}

// r[impl molten.consensus.fast_path_model.evidence]
// r[impl molten.consensus.fast_path_model.nonclaims]
export function runCampaign(outputDirectory, extractedRoot) {
  const here = dirname(fileURLToPath(import.meta.url));
  const modelDirectory = resolve(here, "../tla");
  const output = resolve(outputDirectory);
  // Refuse reuse: a prior successful summary must never mask this invocation.
  mkdirSync(output);
  const started = performance.now();
  const report = {
    schema: "molten.consensus-model-campaign.v1",
    evidenceKind: "finite-model-only",
    status: "failed",
    bounds: { replicas: 3, commands: 2, views: 3 },
    perCheckerLimitMs: CHECKER_TIMEOUT_MS,
    campaignLimitMs: CAMPAIGN_TIMEOUT_MS,
    runs: [],
  };
  const save = () => {
    report.elapsedMs = performance.now() - started;
    writeFileSync(join(output, "campaign.json"), `${JSON.stringify(report, null, 2)}\n`);
  };
  try {
    const archive = readFileSync(join(here, "tla-ts-0.0.10.tgz"));
    requireThat(createHash("sha256").update(archive).digest("hex") === PUBLICATION_SHA256,
      "TLA-ts archive pin mismatch; use the reviewed import procedure, never patch the archive");
    report.tlaTsPublicationSha256 = PUBLICATION_SHA256;
    for (const name of ["ConsensusFastPath.tla", "ConsensusFastPath.cfg", "ConsensusFastPathMissingPromise.cfg"]) {
      copyFileSync(join(modelDirectory, name), join(output, name));
    }
    const model = join(output, "ConsensusFastPath.tla");
    for (const [label, config] of [["baseline", "ConsensusFastPath.cfg"], ["missing-promise", "ConsensusFastPathMissingPromise.cfg"]]) {
      for (const tool of ["tlc", "tla-ts"]) {
        const remaining = CAMPAIGN_TIMEOUT_MS - (performance.now() - started);
        requireThat(remaining > 0, "campaign deadline exceeded");
        const stem = `${label}-${tool}`;
        const args = tool === "tlc"
          ? ["-workers", "1", "-metadir", join(output, `${stem}-states`), "-config", join(output, config), model]
          : [join(here, "check.mjs"), model, join(output, config), resolve(extractedRoot)];
        const executable = tool === "tlc" ? "tlc" : process.execPath;
        const runStarted = performance.now();
        const result = spawnSync(executable, args, {
          encoding: "utf8",
          timeout: Math.min(CHECKER_TIMEOUT_MS, remaining),
          killSignal: "SIGKILL",
          maxBuffer: 64 * 1024 * 1024,
        });
        const elapsedMs = performance.now() - runStarted;
        writeFileSync(join(output, `${stem}.stdout`), result.stdout ?? "");
        writeFileSync(join(output, `${stem}.stderr`), result.stderr ?? "");
        const observed = { label, tool, elapsedMs, exitCode: result.status, signal: result.signal, executable, args };
        report.runs.push(observed);
        save();
        requireThat(!result.error && !result.signal && elapsedMs <= CHECKER_TIMEOUT_MS,
          `${stem}: checker failed or exceeded deadline: ${result.error?.message ?? result.signal ?? elapsedMs}`);
        const summary = tool === "tlc"
          ? parseTlc(`${result.stdout}\n${result.stderr}`, result.status)
          : JSON.parse(result.stdout);
        if (tool === "tla-ts") {
          requireThat(result.status === 0 && summary.tool === "tla-ts@0.0.10", `${stem}: checker API failure`);
        }
        observed.result = summary;
        writeFileSync(join(output, `${stem}.json`), `${JSON.stringify(summary, null, 2)}\n`);
        save();
      }
    }
    report.baseline = validatePair(report.runs[0].result, report.runs[1].result);
    report.mutant = validatePair(report.runs[2].result, report.runs[3].result, true);
    requireThat(performance.now() - started <= CAMPAIGN_TIMEOUT_MS, "campaign exceeded agreed 720 second bound");
    report.status = "passed";
    save();
    return report;
  } catch (error) {
    report.error = `${error.name}: ${error.message}`;
    save();
    throw error;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [output, root] = process.argv.slice(2);
  if (!output || !root) throw new Error("usage: campaign.mjs NEW_OUTPUT_DIRECTORY EXTRACTED_NPM_ROOT");
  runCampaign(output, root);
}
