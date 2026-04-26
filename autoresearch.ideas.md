# Autoresearch Ideas

- Recalibrate the late-stage vendored `openraft` broad Tiger Style benchmark only after the current stable explicit slices plateau: clean-HEAD reruns of the fresh-target-dir full-workspace summary-index command drifted between 268, 273, and 276 findings. Compare current logs against the historical broad logs and either fix the parser/workload definition or promote the loop to a more stable per-file histogram before trusting more broad keeps/discards.
- Revisit `openraft/openraft/src/core/sm/worker.rs` only with a real bounded-channel/backpressure design or a Tiger Style lint improvement. Function- and statement-scoped `#[allow(unbounded_loop)]` did not suppress the custom lint, and wrapper/helper rewrites would risk hiding the unbounded receiver instead of fixing it.
