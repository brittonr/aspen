## Why

Aspen has ~80 crates, 436k lines of Rust, formal verification, 44 NixOS VM tests, and a self-hosting story — but no documentation site. Knowledge is scattered across `docs/`, `AGENTS.md`, `README.md`, inline rustdoc, ADRs, and openspec artifacts. There's no single navigable place to understand the system.

The SSG at `/home/brittonr/git/site` (Onix Computer site) already has a Rust-based static site generator with AsciiDoc, redb caching, Nickel config, live reload, and Nix builds. It handles pages and blog posts. It needs a third content type — docs — to support hierarchical, multi-page documentation with sidebar navigation. Once that exists, Aspen's documentation becomes AsciiDoc content built by the same engine.

## What Changes

- New `docs` content type in the SSG: hierarchical chapters organized by project, rendered with sidebar navigation, previous/next links, and a table of contents
- New `docs.html.erb` template and `docs-layout.html.erb` wrapper for the docs section
- SSG config gains a `docs` section in `site.ncl` for per-project docs settings
- `content/docs/aspen/` directory with AsciiDoc chapters covering architecture, getting started, operator guide, developer guide, and reference
- Doc content structure defined by a `_index.adoc` manifest per section (ordering, titles)
- Nav link added to the site header for `/docs/aspen/`

## Capabilities

### New Capabilities

- `docs-content-type`: Hierarchical documentation content type in the SSG — directory-based sections, sidebar navigation, prev/next links, section manifests, and a docs-specific template
- `aspen-docs-content`: Aspen documentation content — AsciiDoc chapters covering architecture, operations, development, and reference, built by the SSG

### Modified Capabilities

_None._

## Impact

- **SSG crate** (`crates/ssg/`): New `docs.rs` module for doc processing, changes to `build.rs`, `config.rs`, `templates.rs`, `content.rs`
- **Templates**: New `docs.html.erb` and `docs-layout.html.erb`
- **Config**: New `docs` section in `site.ncl`
- **Content**: New `content/docs/aspen/` tree with AsciiDoc files
- **Site nav**: Header gains a "Docs" link
- **Aspen repo**: Doc content authored here, pulled into the site repo (or authored directly in the site repo)
