## 1. SSG Docs Module

- [x] 1.1 Create `crates/ssg/src/docs.rs` with `DocProject`, `DocSection`, `DocChapter` structs
- [x] 1.2 Implement `load_doc_tree()` — parse `content/docs/{project}/_index.adoc` manifests, build hierarchy from `:sections:` and `:order:` attributes
- [x] 1.3 Implement `render_doc_sidebar()` — generate `<nav>` HTML with `<details>`/`<summary>` sections, current page marked active
- [x] 1.4 Implement `process_docs()` — iterate chapters, run asciidoctor, apply cache, render with templates, write to `public/docs/`
- [x] 1.5 Implement prev/next link generation across chapters and section boundaries
- [x] 1.6 Add build warnings for unlisted `.adoc` files in doc sections, errors for missing files listed in `:order:`

## 2. SSG Integration

- [x] 2.1 Add `DocsSection` to `SiteConfig` in `config.rs`, make it optional with defaults
- [x] 2.2 Add `docs` field to `site.ncl` Nickel config
- [x] 2.3 Add `mod docs;` to `main.rs` and call `process_docs()` from `SiteBuilder::build()`
- [x] 2.4 Create `templates/docs-layout.html.erb` — sidebar + content wrapper
- [x] 2.5 Create `templates/docs.html.erb` — chapter content with prev/next links
- [x] 2.6 Add "Docs" link to `templates/layout.html.erb` header nav
- [x] 2.7 Add docs-specific CSS (sidebar layout, active state, prev/next styling, responsive collapse)

## 3. Getting Started Content

- [x] 3.1 Create `content/docs/aspen/_index.adoc` project manifest with all five sections
- [x] 3.2 Create `content/docs/aspen/getting-started/_index.adoc` section manifest
- [x] 3.3 Write `installation.adoc` — Nix setup, `nix develop`, building from source
- [x] 3.4 Write `single-node.adoc` — start a node, CLI interaction, basic KV ops
- [x] 3.5 Write `cluster.adoc` — multi-node cluster via `nix run .#cluster`, verify formation
- [x] 3.6 Write `first-operations.adoc` — KV get/set/scan, coordination primitives quick demo

## 4. Architecture Content

- [x] 4.1 Create `content/docs/aspen/architecture/_index.adoc` section manifest
- [x] 4.2 Write `overview.adoc` — layer diagram, design philosophy, key traits
- [x] 4.3 Write `consensus.adoc` — vendored OpenRaft, redb unified log+state, single-fsync, batching
- [x] 4.4 Write `networking.adoc` — iroh QUIC, ALPN routing, gossip, mDNS, DHT discovery
- [x] 4.5 Write `blob-store.adoc` — iroh-blobs, BLAKE3, content-addressed storage, dedup

## 5. Operator Content

- [x] 5.1 Create `content/docs/aspen/operator/_index.adoc` section manifest
- [x] 5.2 Write `deployment.adoc` — node config flags, cluster formation, NixOS modules
- [x] 5.3 Write `configuration.adoc` — feature flags, environment variables, resource limits
- [x] 5.4 Write `dogfood.adoc` — Forge → CI → Nix build → deploy pipeline, `dogfood-local` commands

## 6. Developer Content

- [x] 6.1 Create `content/docs/aspen/developer/_index.adoc` section manifest
- [x] 6.2 Write `workspace.adoc` — crate layout, key modules, feature flag system
- [x] 6.3 Write `tiger-style.adoc` — coding conventions, FCIS, resource bounds, naming
- [x] 6.4 Write `testing.adoc` — nextest profiles, madsim, proptest, NixOS VM tests, regression policy
- [x] 6.5 Write `verification.adoc` — Verus two-file architecture, writing specs, running verification
- [x] 6.6 Write `ci.adoc` — CI system, flake checks, dogfood pipeline

## 7. Reference Content

- [x] 7.1 Create `content/docs/aspen/reference/_index.adoc` section manifest
- [x] 7.2 Write `adr-index.adoc` — index of all architecture decision records with summaries
- [x] 7.3 Write `cli.adoc` — aspen-cli commands, aspen-node flags, aspen-tui usage
- [x] 7.4 Write `feature-flags.adoc` — all Cargo feature flags with descriptions
- [x] 7.5 Write `limits.adoc` — Tiger Style constants and resource bounds

## 8. Verification

- [x] 8.1 Build the site with `nix build` and verify docs pages are generated under `public/docs/aspen/`
- [x] 8.2 Verify sidebar navigation renders on all docs pages with correct active state
- [x] 8.3 Verify prev/next links work across section boundaries
- [x] 8.4 Verify cached rebuild skips unchanged chapters
- [x] 8.5 Verify missing file in `:order:` produces a build error
- [x] 8.6 Verify unlisted `.adoc` file produces a build warning
