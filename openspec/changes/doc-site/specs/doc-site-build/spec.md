## ADDED Requirements

### Requirement: SSG supports docs content type

The SSG SHALL support a `docs` content type that processes AsciiDoc files from `content/docs/{project}/` into hierarchical documentation pages with sidebar navigation.

#### Scenario: Docs directory is processed during build

- **WHEN** `content/docs/aspen/` exists with `_index.adoc` and chapter files
- **THEN** the SSG build produces HTML pages under `public/docs/aspen/`

#### Scenario: No docs directory is a no-op

- **WHEN** `content/docs/` does not exist
- **THEN** the SSG build completes without errors and produces no docs output

### Requirement: Section manifests control ordering

Each docs section SHALL have a `_index.adoc` file with an `:order:` attribute that defines the chapter sequence. Chapters not listed in `:order:` SHALL be excluded from the build.

#### Scenario: Chapters render in manifest order

- **WHEN** `_index.adoc` contains `:order: install, single-node, cluster`
- **THEN** the sidebar and prev/next links follow that order regardless of filesystem sort

#### Scenario: Unlisted chapter is excluded

- **WHEN** a `.adoc` file exists in a section directory but is not listed in `:order:`
- **THEN** no HTML page is generated for that file and a build warning is emitted

#### Scenario: Listed file missing is an error

- **WHEN** `:order:` lists a slug that has no corresponding `.adoc` file
- **THEN** the build fails with an error identifying the missing file

### Requirement: Project manifest controls section ordering

The project-level `_index.adoc` SHALL have a `:sections:` attribute that defines the section sequence and provides the project landing page content.

#### Scenario: Sections render in manifest order

- **WHEN** `content/docs/aspen/_index.adoc` contains `:sections: getting-started, architecture, operator`
- **THEN** the sidebar lists sections in that order

#### Scenario: Project landing page is generated

- **WHEN** the build processes a docs project
- **THEN** `public/docs/aspen/index.html` is generated from the project `_index.adoc` body content

### Requirement: Sidebar navigation renders on every docs page

Every docs page SHALL include a sidebar showing the full navigation tree with the current page highlighted.

#### Scenario: Sidebar shows all sections and chapters

- **WHEN** a docs chapter page is rendered
- **THEN** the page includes a `<nav>` element listing all sections and their chapters

#### Scenario: Current page is marked active

- **WHEN** a reader is viewing `/docs/aspen/architecture/consensus/`
- **THEN** the sidebar marks the "Consensus" chapter and "Architecture" section as active

#### Scenario: Sections are collapsible

- **WHEN** the sidebar renders
- **THEN** each section uses `<details>`/`<summary>` elements so sections can be collapsed, with the current section open by default

### Requirement: Previous/next navigation between chapters

Each docs chapter page SHALL include previous and next links for sequential navigation within and across sections.

#### Scenario: Middle chapter has both links

- **WHEN** a chapter is not the first or last in the project
- **THEN** the page includes both "Previous" and "Next" links to adjacent chapters

#### Scenario: First chapter has no previous link

- **WHEN** a chapter is the first in the project
- **THEN** only a "Next" link is shown

#### Scenario: Navigation crosses section boundaries

- **WHEN** a chapter is the last in a section and another section follows
- **THEN** the "Next" link points to the first chapter of the next section

### Requirement: Docs use the existing caching system

Doc chapter builds SHALL use the existing redb cache with content hashing, matching the caching behavior of pages and posts.

#### Scenario: Cached chapter is not re-rendered

- **WHEN** a chapter's content has not changed since the last build
- **THEN** the cached HTML is reused and asciidoctor is not invoked for that chapter

#### Scenario: Changed chapter is rebuilt

- **WHEN** a chapter's content hash differs from the cached version
- **THEN** asciidoctor processes the file and the cache is updated

### Requirement: Docs module is separate from build.rs

Doc processing logic SHALL live in `crates/ssg/src/docs.rs` as a separate module, called from `SiteBuilder::build()`.

#### Scenario: docs.rs module exists

- **WHEN** the SSG crate is compiled
- **THEN** `crates/ssg/src/docs.rs` is included as a module

#### Scenario: Builder calls process_docs

- **WHEN** `SiteBuilder::build()` runs
- **THEN** it calls `process_docs()` to handle docs content, after processing pages

### Requirement: Docs config section in site.ncl

The SSG config SHALL support an optional `docs` section in `site.ncl` with title and description fields.

#### Scenario: Config with docs section

- **WHEN** `site.ncl` contains `docs = { title = "Documentation" }`
- **THEN** `SiteConfig` parses the `docs` field into a `DocsSection` struct

#### Scenario: Config without docs section

- **WHEN** `site.ncl` has no `docs` field
- **THEN** `SiteConfig` uses defaults and the build proceeds normally

### Requirement: URL structure follows directory hierarchy

Doc pages SHALL be output at URLs matching `docs/{project}/{section}/{chapter}/index.html`.

#### Scenario: Chapter URL matches path

- **WHEN** `content/docs/aspen/architecture/consensus.adoc` is processed
- **THEN** the output is written to `public/docs/aspen/architecture/consensus/index.html`

#### Scenario: Section landing URL

- **WHEN** a section `_index.adoc` is processed
- **THEN** the output is written to `public/docs/aspen/architecture/index.html`
