## ADDED Requirements

### FLKEVAL-1: Evaluate flake via call-flake.nix

The system MUST evaluate flakes by invoking an embedded copy of Nix's `call-flake.nix` expression via snix-eval. The expression receives three arguments: the flake.lock JSON as a string, the pre-resolved overrides attrset, and a `fetchTreeFinal` function. The evaluation MUST produce the flake's `outputs` attrset.

### FLKEVAL-2: Extract Derivation from evaluation

After evaluating the flake outputs expression and navigating to the requested attribute's `.drvPath`, the system MUST extract the `Derivation` object from snix-glue's `KnownPaths`. The derivation MUST be equivalent to what `nix eval --raw .#<attr>.drvPath` followed by `Derivation::from_aterm_bytes` would produce.

### FLKEVAL-3: Navigate flake attribute paths

The system MUST support navigating dot-separated attribute paths (e.g., `packages.x86_64-linux.default`) through the flake outputs. Attribute names containing hyphens or starting with digits MUST be quoted.

### FLKEVAL-4: fetchTreeFinal stub

The `fetchTreeFinal` argument passed to `call-flake.nix` MUST throw an error indicating the input type and name when called. All inputs SHOULD be resolved via overrides before evaluation begins, making this a dead code path in the success case.

### FLKEVAL-5: Self input handling

The root flake source (the checked-out repository) MUST be passed as the root node's override with `outPath` pointing to the flake source directory. This provides the `self` input to `flake.outputs`.

### FLKEVAL-6: Fallback to subprocess

When in-process evaluation fails for any reason (IFD, unsupported builtins, fetchTreeFinal invoked, evaluation error), the system MUST fall back to the existing `nix eval --raw .drvPath` subprocess path. The fallback MUST be gated by the `nix-cli-fallback` feature flag.

### FLKEVAL-7: Log zero-subprocess success

When flake evaluation succeeds fully in-process (no subprocess), the system MUST log a message containing "zero subprocesses" at INFO level, matching the existing npins native eval log pattern.

### FLKEVAL-8: Source size limits

The generated evaluation expression (call-flake.nix + lockfile JSON + overrides) MUST not exceed 2 MB. Flake.lock files larger than 1 MB MUST be rejected.
