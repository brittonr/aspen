## 1. SSG Docs Module

- [ ] 1.1 Create `crates/ssg/src/docs.rs` with `DocProject`, `DocSection`, `DocChapter` structs
- [ ] 1.2 Implement `load_doc_tree()` — parse `content/docs/{project}/_index.adoc` manifests, build hierarchy from `:sections:` and `:order:` attributes
- [ ] 1.3 Implement `render_doc_sidebar()` — generate `<nav>` HTML with `<details>`/`<summary>` sections, current page marked active
- [ ] 1.4 Implement `process_docs()` — iterate chapters, run asciidoctor, apply cache, render with templates, write to `public/docs/`
- [ ] 1.5 Implement prev/next link generation across chapters and section boundaries
- [ ] 1.6 Add build warnings for unlisted `.adoc` files in doc sections, errors for missing files listed in `:order:`

## 2. SSG Integration

- [ ] 2.1 Add `DocsSection` to `SiteConfig` in `config.rs`, make it optional with defaults
- [ ] 2.2 Add `docs` field to `site.ncl` Nickel config
- [ ] 2.3 Add `mod docs;` to `main.rs` and call `process_docs()` from `SiteBuilder::build()`
- [ ] 2.4 Create `templates/docs-layout.html.erb` — sidebar + content wrapper
- [ ] 2.5 Create `templates/docs.html.erb` — chapter content with prev/next links
- [ ] 2.6 Add "Docs" link to `templates/layout.html.erb` header nav
- [ ] 2.7 Add docs-specific CSS (sidebar layout, active state, prev/next styling, responsive collapse)

## 3. Getting Started Content

- [ ] 3.1 Create `content/docs/aspen/_index.adoc` project manifest with all five sections
- [ ] 3.2 Create `content/docs/aspen/getting-started/_index.adoc` section manifest
- [ ] 3.3 Write `installation.adoc` — Nix setup, `nix develop`, building from source
- [ ] 3.4 Write `single-node.adoc` — start a node, CLI interaction, basic KV ops
- [ ] 3.5 Write `cluster.adoc` — multi-node cluster via `nix run .#cluster`, verify formation
- [ ] 3.6 Write `first-operations.adoc` — KV get/set/scan, coordination primitives quick demo

## 4. Architecture Content

- [ ] 4.1 Create `content/docs/aspen/architecture/_index.adoc` section manifest
- [ ] 4.2 Write `overview.adoc` — layer diagram, design philosophy, key traits
- [ ] 4.3 Write `consensus.adoc` — vendored OpenRaft, redb unified log+state, single-fsync, batching
- [ ] 4.4 Write `networking.adoc` — iroh QUIC, ALPN routing, gossip, mDNS, DHT discovery
- [ ] 4.5 Write `blob-store.adoc` — iroh-blobs, BLAKE3, content-addressed storage, dedup

## 5. Operator Content

- [ ] 5.1 Create `content/docs/aspen/operator/_index.adoc` section manifest
- [ ] 5.2 Write `deployment.adoc` — node config flags, cluster formation, NixOS modules
- [ ] 5.3 Write `configuration.adoc` — feature flags, environment variables, resource limits
- [ ] 5.4 Write `dogfood.adoc` — Forge → CI → Nix build → deploy pipeline, `dogfood-local` commands

## 6. Developer Content

- [ ] 6.1 Create `content/docs/aspen/developer/_index.adoc` section manifest
- [ ] 6.2 Write `workspace.adoc` — crate layout, key modules, feature flag system
- [ ] 6.3 Write `tiger-style.adoc` — coding conventions, FCIS, resource bounds, naming
- [ ] 6.4 Write `testing.adoc` — nextest profiles, madsim, proptest, NixOS VM tests, regression policy
- [ ] 6.5 Write `verification.adoc` — Verus two-file architecture, writing specs, running verification
- [ ] 6.6 Write `ci.adoc` — CI system, flake checks, dogfood pipeline

## 7. Reference Content

- [ ] 7.1 Create `content/docs/aspen/reference/_index.adoc` section manifest
- [ ] 7.2 Write `adr-index.adoc` — index of all architecture decision records with summaries
- [ ] 7.3 Write `cli.adoc` — aspen-cli commands, aspen-node flags, aspen-tui usage
- [ ] 7.4 Write `feature-flags.adoc` — all Cargo feature flags with descriptions
- [ ] 7.5 Write `limits.adoc` — Tiger Style constants and resource bounds

## 8. Verification

- [ ] 8.1 Build the site with `nix build` and verify docs pages are generated under `public/docs/aspen/`
- [ ] 8.2 Verify sidebar navigation renders on all docs pages with correct active state
- [ ] 8.3 Verify prev/next links work across section boundaries
- [ ] 8.4 Verify cached rebuild skips unchanged chapters
- [ ] 8.5 Verify missing file in `:order:` produces a build error
- [ ] 8.6 Verify unlisted `.adoc` file produces a build warning
