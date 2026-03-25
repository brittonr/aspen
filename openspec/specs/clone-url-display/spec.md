## ADDED Requirements

### Requirement: Clone URL on repo overview

The repo overview page SHALL display the `git clone` command with the full `aspen://` URL so users can clone the repository.

#### Scenario: Repo overview shows clone command

- **WHEN** user views a repo overview page
- **THEN** the page SHALL display a copyable `git clone aspen://{ticket}/{repo_id} {repo_name}` command

### Requirement: Markdown rendering in issues and patches

Issue and patch bodies and comments SHALL render as markdown instead of plain text.

#### Scenario: Issue body with markdown

- **WHEN** an issue body contains markdown syntax (headings, links, code blocks)
- **THEN** the rendered page SHALL display formatted HTML, not raw markdown text

#### Scenario: Comment with markdown

- **WHEN** a comment body contains markdown
- **THEN** the rendered comment SHALL display formatted HTML
