# Change-local acceptance

a[readme-default-run.commands] README `cargo run -- …` commands resolve to the molten binary; three documented commands run without `--bin`.
a[readme-default-run.plans] `Cargo.lock`, `build-plan.json`, and `release-policy-build-plan.json` are unchanged, and the unit2nix staleness check still passes.
a[readme-default-run.state] The README current-state paragraph reports dated, measured counts for the current head.
