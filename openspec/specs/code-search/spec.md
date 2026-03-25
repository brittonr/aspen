## ADDED Requirements

### Requirement: Code search page

Each repo SHALL have a search page at `/{repo_id}/search` that accepts a query parameter `q` and returns matching file contents from HEAD.

#### Scenario: Search with matches

- **WHEN** user searches for "println" in a repo containing Rust files with that string
- **THEN** the page SHALL list each matching file path with the matching lines and 2 lines of surrounding context

#### Scenario: Search with no matches

- **WHEN** user searches for a string not present in any file
- **THEN** the page SHALL display "No results for {query}"

#### Scenario: Query too short

- **WHEN** user submits a query shorter than 2 characters
- **THEN** the page SHALL display a message asking for at least 2 characters

### Requirement: Search bounds

The search SHALL be bounded to prevent resource exhaustion.

#### Scenario: Many files in repo

- **WHEN** a repo has more than 500 files
- **THEN** the search SHALL examine at most 500 files and indicate if results are incomplete

#### Scenario: Large file skipped

- **WHEN** a file exceeds 256KB
- **THEN** it SHALL be skipped without error

#### Scenario: Many matches

- **WHEN** more than 50 matches are found
- **THEN** the page SHALL show the first 50 and indicate more exist

### Requirement: Search box in repo navigation

Each repo page SHALL include a search input in the navigation tabs that submits to the search page.

#### Scenario: Submit search from any repo page

- **WHEN** user types a query and presses enter in the search input
- **THEN** the browser SHALL navigate to `/{repo_id}/search?q={query}`

### Requirement: Result links to file with line anchor

Each search result line SHALL link to the file viewer at the matching line.

#### Scenario: Click a result line

- **WHEN** user clicks a matching line in search results
- **THEN** the browser SHALL navigate to `/{repo_id}/blob/{ref}/{path}#L{line_number}`
