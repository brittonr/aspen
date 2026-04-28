## Implementation

- [x] I1 Add a Forge-local Onix-inspired CSS token layer in `crates/aspen-forge-web/src/templates.rs` covering surfaces, typography, borders, shadows, status colors, code surfaces, and focus states.
- [x] I2 Retheme common Forge web components through those tokens: page shell, header/nav, repo cards, tabs, tables, buttons, forms, badges, alerts, code/log blocks, CI stages, and job rows.
- [x] I3 Document `../onix-site/` as the Forge web UI reference in `README.md`.

## Verification

- [x] V1 Run `cargo nextest run -p aspen-forge-web` and save the transcript under this change's `evidence/` directory.
- [x] V2 Run `git diff --check` and save the transcript under this change's `evidence/` directory.
- [x] V3 Run `scripts/openspec-preflight.sh onix-inspired-forge-web-ui` and save the transcript under this change's `evidence/` directory.
