## Context

The SSG at `/home/brittonr/git/site` is a custom Rust static site generator with:

- **Content types**: Pages (`content/pages/*.adoc`) and Posts (`content/posts/*.adoc`)
- **Rendering**: AsciiDoc via asciidoctor, ERB-style templates, heading anchors
- **Caching**: redb-backed incremental builds with content hashing
- **Config**: Nickel (`site.ncl`) parsed into typed Rust structs
- **Build**: Nix flake with crane, live reload via polling

Pages are flat (one `.adoc` → one URL). Posts are chronological with tags, pagination, and feeds. Neither supports hierarchical documentation with sidebar navigation — the kind of structure needed for a project like Aspen with multiple sections, chapters, and cross-references.

## Goals / Non-Goals

**Goals:**

- Add a `docs` content type to the SSG that supports multi-level hierarchy (project → section → chapter)
- Render docs with a sidebar showing the full navigation tree, highlighting the current page
- Previous/next navigation between chapters within a section
- Per-section ordering via `_index.adoc` manifests (not filesystem sort)
- Reuse the existing caching, asciidoctor pipeline, and template system
- Write Aspen's docs as AsciiDoc content in `content/docs/aspen/`

**Non-Goals:**

- Search (can add later with a client-side index)
- Versioned docs (one version for now)
- Auto-generated API docs from rustdoc (separate concern)
- Multi-project docs landing page (only Aspen for now, structure supports adding more later)
- PDF/epub export

## Decisions

### 1. Directory-based hierarchy under `content/docs/{project}/`

**Choice**: `content/docs/aspen/` with subdirectories for sections, each containing chapter `.adoc` files and a `_index.adoc` manifest.

```
content/docs/aspen/
├── _index.adoc              # project-level manifest (title, description, section order)
├── getting-started/
│   ├── _index.adoc          # section manifest (chapter order, section title)
│   ├── installation.adoc
│   ├── single-node.adoc
│   └── cluster.adoc
├── architecture/
│   ├── _index.adoc
│   ├── overview.adoc
│   └── consensus.adoc
└── ...
```

**Alternatives considered**:

- **Single flat directory** — no hierarchy, doesn't scale to 20+ chapters across 5 sections.
- **YAML/TOML manifest** — introduces a new config format. `_index.adoc` keeps everything in AsciiDoc and matches Hugo/Zola conventions.

**Rationale**: Directory names become URL segments (`/docs/aspen/architecture/consensus/`). `_index.adoc` controls ordering and provides section landing page content. Mirrors how the existing pages/posts directories work, extended one level deeper.

### 2. `_index.adoc` manifest format

Each `_index.adoc` has standard AsciiDoc attributes for metadata:

```asciidoc
= Getting Started
:description: Install Aspen and run your first cluster.
:order: installation, single-node, cluster, first-operations
```

The `:order:` attribute is a comma-separated list of slugs defining chapter sequence. Chapters not listed are excluded from the build (draft behavior). The section title comes from the `= Title` line.

Project-level `_index.adoc` uses `:sections:` instead:

```asciidoc
= Aspen Documentation
:description: Distributed systems orchestration in Rust.
:sections: getting-started, architecture, operator, developer, reference
```

### 3. Docs-specific templates

**Choice**: Two new templates:

- `docs-layout.html.erb` — outer wrapper with sidebar + content area, replaces the standard page layout for docs
- `docs.html.erb` — chapter content rendering (title, body, prev/next links)

The sidebar is rendered server-side as a nested `<nav>` with the current page marked active. No client-side JS tree — pure HTML/CSS with details/summary for collapsible sections.

**Rationale**: Matches the existing template pattern. Server-rendered nav means no JS dependency and instant page loads.

### 4. New module `crates/ssg/src/docs.rs`

**Choice**: Separate module for docs processing rather than extending `build.rs` inline.

Contains:

- `DocProject` / `DocSection` / `DocChapter` structs
- `load_doc_tree()` — reads `content/docs/{project}/` and builds the hierarchy
- `render_doc_sidebar()` — generates the sidebar HTML from the tree
- `process_docs()` — called from `SiteBuilder::build()`, parallel asciidoctor like posts

**Rationale**: `build.rs` is already 3,188 lines. Docs processing is self-contained enough to isolate. The builder calls `process_docs()` the same way it calls `process_pages()`.

### 5. Config extension

Add a `docs` section to `SiteConfig`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DocsSection {
    pub title: Option<String>,
    #[serde(default)]
    pub description: String,
}
```

And in `site.ncl`:

```nickel
docs = {
  title = "Documentation",
  description = "Project documentation.",
},
```

Minimal config — most behavior comes from `_index.adoc` manifests per project.

### 6. URL structure

```
/docs/aspen/                          → project landing (from _index.adoc)
/docs/aspen/getting-started/          → section landing (from section _index.adoc)
/docs/aspen/getting-started/install/  → chapter page
```

Each chapter renders to `{slug}/index.html` matching the existing page convention.

## Risks / Trade-offs

- **[SSG complexity]** → Adding a third content type with hierarchy grows the codebase. Mitigation: isolated in `docs.rs`, reuses existing asciidoctor/cache/template infrastructure.
- **[Content maintenance]** → Docs authored in the site repo mean Aspen changes and doc updates are in different repos. Mitigation: this is normal for hosted documentation. CI can verify doc builds when aspen changes.
- **[Sidebar rendering cost]** → Server-rendering the full nav tree on every page means N copies of the sidebar HTML. Mitigation: sidebar is small (a few KB), gzips well, and avoids any JS framework.
- **[_index.adoc ordering]** → Manual ordering via `:order:` can drift from actual files. Mitigation: build warns on files present but not listed (potential drafts) and errors on listed files that don't exist.
