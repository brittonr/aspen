## Context

`validate_probe_results` iterates an externally supplied slice. Every probe can carry one status-condition reference, and the method clones all present references into its result. The same included lifecycle module already enforces `MAX_REFS = 1_024` for canonical lifecycle receipt reference lists, but this probe entry point omitted any admission preflight.

## Decisions

### Reuse the lifecycle receipt reference envelope

Check the probe count against `MAX_REFS` before allocating output or validating and cloning the first probe reference. This bounds the worst-case result with the existing receipt limit and does not invent a second quota. Exactly 1,024 probes remain eligible; 1,025 deny with an invalid-harness error. Duplicate status references remain in input order as before; normal evidence-ref and generation validation is unchanged for admitted counts.

### Exercise the production API externally

Retain a permanent exact/+1 source regression which failed before the fix. Compile a disposable separate Rust binary against the real molten library to observe both accepted and denied boundary values. Mirror only lifecycle source, regression, handoff evidence, and this change-local delta into the private composition; leave accepted specs and the existing 59 change packages untouched.

## Risks and non-claims

Inputs with more than 1,024 probes now deny even when some probes omit a status reference. This is deliberate: every probe could carry one and this boundary must reject before copying. The change does not bound arbitrary input construction outside this borrowed-slice API, rewrite restart decisions, or establish full strict-ci acceptance.
