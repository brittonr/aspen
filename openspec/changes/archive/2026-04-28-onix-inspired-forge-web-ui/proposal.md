## Why

Forge web has functional pages, but its visual language currently drifts from the Onix/Aspen ecosystem. Using `../onix-site/` as the design reference gives Forge a coherent onyx/phosphor UI language while keeping the frontend self-contained and Iroh-only.

## What Changes

- Adopt an Onix-inspired token layer in the Forge web stylesheet: onyx/bone surfaces, phosphor signal accent, hard borders, square corners, and role-specific font stacks.
- Restyle Forge web chrome, cards, tables, badges, buttons, tabs, code/log surfaces, and CI panels to use those tokens consistently.
- Document `../onix-site/` as the visual reference for future Forge web UI work.
- No runtime dependency on `../onix-site/`; it is a design reference only.

## Capabilities

### New Capabilities

- `forge-web-ui-style`: Forge web pages share an Onix-inspired design system derived from `../onix-site/` reference files.

### Modified Capabilities

- None.

## Impact

- **Files**: `crates/aspen-forge-web/src/templates.rs`, `README.md`, OpenSpec artifacts.
- **APIs**: No RPC or wire API changes.
- **Dependencies**: No new crate or runtime dependency; the UI remains rendered by Maud templates and embedded CSS.
- **Testing**: Existing `aspen-forge-web` template and patchbay tests continue to pass; HTML assertions cover core routes and CI UI pieces.
