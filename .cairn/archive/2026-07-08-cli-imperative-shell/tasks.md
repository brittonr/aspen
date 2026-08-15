## Tasks

- [x] [serial] r[molten.modularity.cli_shell.thin_shell] Selected the boundary-planning workflow and documented the typed command-core input/output shape in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.modularity.cli_shell.typed_core] Moved deterministic command decisions into `molten-core` planners callable without Clap, filesystem state, stdout, stderr, process exits, network services, or live adapter execution.
- [x] [parallel] r[molten.modularity.cli_shell.compatible_ux] Preserved existing command names, flags, and output contracts; root compatibility re-exports continue through `molten::core_api` and `molten::prelude`.
- [x] [parallel] r[molten.modularity.cli_shell.tests] Added root positive and negative prelude tests for valid command-core inputs and denied plans with no effects.
- [x] [serial] r[molten.modularity.cli_shell.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
