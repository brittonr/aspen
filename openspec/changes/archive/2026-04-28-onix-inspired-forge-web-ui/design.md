## Context

Forge web renders server-side HTML with Maud and embeds its stylesheet in `templates.rs`. `../onix-site/` contains the current Onix design direction: grayscale-first onyx/bone surfaces, phosphor accent as signal, square neo-brutalist borders, role-specific typography, and composed site UI kit patterns.

The Forge web UI should borrow the design language without introducing a runtime dependency on a sibling checkout. The reference files are evidence and inspiration, not source files to import.

## Goals / Non-Goals

**Goals:**

- Create a Forge web CSS token layer that mirrors the Onix reference vocabulary.
- Re-theme existing Forge web components through tokens and shared component rules.
- Keep HTML routes, URLs, RPC calls, and Maud rendering architecture unchanged.
- Document the reference path so future UI changes use the same source of truth.

**Non-Goals:**

- Rebuild Forge web as React or copy JSX components from `../onix-site/ui_kits/site/`.
- Add network-loaded fonts or require external CSS at runtime.
- Change Forge web product flows, route structure, or RPC/API behavior.
- Implement full theme switching in this pass.

## Decisions

### 1. Embed a local Onix-inspired token layer

**Choice:** Define Forge-local CSS custom properties in the embedded stylesheet.

**Rationale:** Forge web is self-contained and works in isolated cluster environments. Local tokens allow consistency without coupling builds/tests to a sibling repository.

**Alternative:** Import `../onix-site/design-system/colors_and_type.css` directly. Rejected because sibling paths are not guaranteed in packaged builds and would violate the design-only reference requirement.

**Implementation:** Update `BASE_CSS` in `crates/aspen-forge-web/src/templates.rs` to define token groups for surfaces, type, borders, shadows, spacing, and status colors, then route component rules through those tokens.

### 2. Preserve Maud templates and component structure

**Choice:** Keep existing HTML and route handlers mostly intact; retheme CSS selectors already used by templates.

**Rationale:** Existing route tests and patchbay end-to-end tests assert content, navigation, and CI behavior. CSS-only re-theming minimizes risk while delivering the requested UI direction.

**Alternative:** Restructure templates around imported Onix components. Rejected because it would be larger, riskier, and not aligned with the current server-rendered architecture.

### 3. Treat phosphor as signal, not fill

**Choice:** Use the accent color for focus rings, status dots, highlights, links, and small badges while surfaces remain grayscale.

**Rationale:** This follows the Onix design reference where phosphor is a single-pixel signal accent rather than a broad background fill.

**Implementation:** Map success/running/link/focus states to accent variables, use warning/error variables sparingly, and keep large panels on onyx/graphite surfaces.

## Risks / Trade-offs

**[Visual regression hidden by content-only tests]** → Keep changes CSS-scoped and run the full `aspen-forge-web` test package. Future visual screenshots can be added if Forge web gains browser-level regression tests.

**[Over-copying Onix site]** → Document reference files and copy design principles/tokens, not JSX or runtime assets.

**[Accessibility contrast]** → Use high-contrast onyx/bone foreground/background tokens and keep focus outlines visible.
