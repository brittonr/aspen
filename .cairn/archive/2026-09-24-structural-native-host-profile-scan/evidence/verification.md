# Verification: Structural native host profile scan

Base: `origin/molten` `4e31cee55167a38978961faac5c46476aca6f5ac`.

## Baseline failure

- On the base, `checks.x86_64-linux.native-system-extension-host-profile` exits 1 with no output. A `set -x` rebuild
  stops right after `rg -Fq 'materialized_output: Option<NativeCallbackValue>' src/system_extension/canonical.rs`.
- Import codemod `18a59c2b0` rewrote both `materialized_output` fields in `src/system_extension/canonical.rs` to
  `Option<super::NativeCallbackValue>`.

## Positive

- Probe regex `\bmaterialized_output\s*:\s*Option\s*<\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*NativeCallbackValue\s*>`
  (ripgrep `-P`) matches `Option<NativeCallbackValue>`, `Option<super::NativeCallbackValue>`, and
  `Option<crate::system_extension::NativeCallbackValue>`.
- `nix build .#checks.x86_64-linux.native-system-extension-host-profile` on the change branch: exit 0.

## Negative

Each case overrides the check's `src` with a mutated copy of the branch tree through `overrideAttrs`:

- `Option<String>` instead of the value type: exit 1, "canonical effect completion no longer carries an optional
  materialized NativeCallbackValue".
- Field renamed to `materialized_output_ref`: exit 1, same diagnostic.
- `NativeHostJournal` removed from `journal.rs`: exit 1, "native host profile source requirement missing:
  'NativeHostJournal' in src/system_extension/native_host/journal.rs".
- Regex probe alone: `Option<String>` and `materialized_output_ref` do not match.

Logs: `/home/brittonr/git/OnixResearch/target/aspen-gate-blockers/d-host-profile-check.txt`, `/home/brittonr/git/OnixResearch/target/aspen-gate-blockers/d-negative-probes.txt`.

## Non-claims

The check stays a text-level structure probe. It does not prove that the field is populated correctly at runtime.
The effect-completion tests own that.
