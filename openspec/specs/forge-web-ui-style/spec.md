# forge-web-ui-style Specification

## Purpose
TBD - created by archiving change onix-inspired-forge-web-ui. Update Purpose after archive.
## Requirements
### Requirement: Onix-inspired Forge web visual system

Forge web SHALL render pages using a self-contained Onix-inspired visual system based on the `../onix-site/` design reference. The implementation SHALL preserve server-rendered Maud templates and SHALL NOT require loading files from `../onix-site/` at runtime.

#### Scenario: Shared token layer

- **GIVEN** a Forge web page is rendered
- **WHEN** the embedded stylesheet is applied
- **THEN** shared CSS tokens SHALL define page, surface, foreground, muted, border, accent, warning, error, success, inverse, code, shadow, font, spacing, and timing values
- **AND** those tokens SHALL use the onyx/bone/phosphor direction from `../onix-site/design-system/colors_and_type.css`

#### Scenario: Common components use the shared tokens

- **GIVEN** a Forge web page contains common chrome such as navigation tabs, cards, tables, buttons, badges, forms, code blocks, log output, CI stages, job rows, or alerts
- **WHEN** the page is rendered
- **THEN** those components SHALL use the shared tokens for color, borders, typography, spacing, and focus/hover states rather than ad-hoc GitHub-like colors

#### Scenario: Reference remains design-only

- **GIVEN** the Forge web crate is built or tested without a sibling `../onix-site/` checkout
- **WHEN** templates and tests compile
- **THEN** no runtime, Cargo, Nix, or filesystem dependency on `../onix-site/` SHALL be required

### Requirement: UI reference documentation

The project README SHALL identify `../onix-site/` as the Forge web UI reference and list the reference files future contributors should inspect before changing Forge web styling.

#### Scenario: Contributor finds the UI reference

- **WHEN** a contributor reads `README.md`
- **THEN** the README SHALL include a `References` entry for `../onix-site/`
- **AND** the entry SHALL name `design-system/colors_and_type.css` and `ui_kits/site/` as the key reference locations
