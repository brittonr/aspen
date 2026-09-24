# Change-local acceptance

a[bind-metadata-free-git-source-hashes.fetch] The five fixed-output git sources (ChaosControl, durable-authority-state, bounded-http, content-identity, Choregraph) realize and pass `nix-store --realise --check` with the plan hashes.
a[bind-metadata-free-git-source-hashes.binding] Every git source hash in both generated plans equals a revision-qualified SRI entry in `crate-hashes.json`, and `crate-hashes.json` covers every non-Radicle Cargo.lock git source.
a[bind-metadata-free-git-source-hashes.drift-denied] `scripts/git-source-hashes.sh --check` and `checks.x86_64-linux.git-source-hash-binding` fail on an unbound, unqualified, or drifted hash.
a[bind-metadata-free-git-source-hashes.procedure] `docs/reproducible-dependencies.md` records the hash-binding step and the exact unit2nix regeneration commands, and regeneration with the pinned unit2nix reproduces both plans apart from `workspaceRoot`.
a[bind-metadata-free-git-source-hashes.unblocked] The previously blocked checks build with the corrected hashes.
