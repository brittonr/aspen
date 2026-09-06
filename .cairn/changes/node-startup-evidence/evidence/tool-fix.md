# Octet bare-todo fix candidate

The operator requested continuation after the bare-todo defect report.
The fix belongs to Octet, not the Molten verifier or startup shell.

Octet implementation: `ca08a7c73c27f07b11c5f0580d3d033b696bf7a0`.
Octet evidence checkpoint: `c9b06bcf565c51d4a77d210e61b69ae51db9df25`.
Published branch: `OnixResearch/octet`, `fix/no-todo-expansion-shapes`.
Evidence path: `verification/no-todo-expansion/verification.md` at that checkpoint.

The fix removes the expanded-block requirement and deduplicates recognized macro callsites.
Four UI snapshots pass with the candidate library. The old library fails the same new expectations.
Two helper tests, library Clippy with `-D warnings`, and pinned formatting for changed Rust owners pass.
The strict bare-todo probe now exits two with two error-level findings instead of a false-clean zero.
Its unchanged empty fixture still exits zero.

The strict probe deliberately combines the old pinned CLI and the candidate library.
It is diagnostic evidence, not a newly admitted tool cohort.
The full UI suite still fails on the pre-existing `crossbeam-channel` fixture/vendor version mismatch.
Both baseline and candidate reach that same blocker. No fixture lock or dependency pin changed.

The candidate library was also built from the immutable implementation revision.
Its output is `/nix/store/1wmd44w5qrpazl97g8rv0dfkaf3wzb9j-octet-0.1.0/lib/liboctet.so`.
Its BLAKE3 is `10919bdd10b0049c1b1113f9757dc563441c6709b91efb02323ff5d32489447b`.
The complete source NAR is `sha256-hHIGUJ69HJWKBue0hlTC9sLKb7OQIDlEL884BgGbyTA=`.
These are candidate identities, not replacements for the existing approved expectations.

Molten's Octet pin remains `fc38f59330b626961d166febfdf1a5aa6575460f`.
No consumer manifest, lockfile, policy, startup guard, or task checkbox changed.
No complete Molten gate, VM, native replay, Stage0, compiler rebuild, package rebuild, or release promotion occurred.
The existing compiler, driver, Mantle, native archive, guarded Molten binary, and original blob fixture retain their measured bytes.

Next is explicit review of the new Octet revision and its remaining acceptance limits.
A future consumer update needs a distinct tool cohort and genuine complete-source execution evidence.
The existing startup and VM guards remain mandatory until lifecycle admission is implemented and proved.
