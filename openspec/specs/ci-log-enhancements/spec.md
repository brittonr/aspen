## ADDED Requirements

### Requirement: Line numbers in job log viewer

The job log viewer SHALL display line numbers for each line of log output. Line numbers SHALL be rendered in a left-aligned column with muted color, same style as the existing source code viewer. Line numbers SHALL be sequential across all displayed chunks, starting from 1.

#### Scenario: Log with multiple lines

- **WHEN** user views a job log page with 150 lines of output
- **THEN** each line SHALL have a sequential line number from 1 to 150 in a left column
- **AND** line numbers SHALL be non-selectable (user-select: none)

#### Scenario: Log with ANSI colors and line numbers

- **WHEN** user views a job log with ANSI escape codes
- **THEN** line numbers SHALL appear alongside the ANSI-converted HTML content
- **AND** ANSI color rendering SHALL NOT be affected by the line number column

### Requirement: Full output mode

The job log viewer SHALL support a "full output" mode toggled by a `?full=1` query parameter. When active, the handler SHALL call `CiGetJobOutput` instead of `CiGetJobLogs` and render the complete stdout and stderr. The full output SHALL be capped at 1 MB of displayed text.

#### Scenario: Toggle full output mode

- **WHEN** user clicks "View full output" link on a job log page
- **THEN** the page SHALL reload with `?full=1` appended to the URL
- **AND** the log viewer SHALL display the complete stdout followed by stderr (if present)
- **AND** a "View chunked logs" link SHALL be available to return to the default view

#### Scenario: Full output exceeds 1 MB

- **WHEN** the combined stdout and stderr exceed 1 MB
- **THEN** the display SHALL be truncated at 1 MB
- **AND** a notice SHALL indicate that output was truncated
- **AND** the notice SHALL suggest using `aspen-cli ci output <run_id> <job_id>` for the complete output

#### Scenario: Full output for a running job

- **WHEN** user views full output for a job that is still running
- **THEN** the page SHALL display whatever output is available so far
- **AND** the page SHALL auto-refresh to show new output

### Requirement: Stderr separation in full output mode

In full output mode, stdout and stderr SHALL be visually separated. Stderr SHALL be rendered in a distinct section with a "stderr" header and a different background tint (darker red-tinted background) to distinguish it from stdout.

#### Scenario: Job with both stdout and stderr

- **WHEN** user views full output for a job that produced both stdout and stderr
- **THEN** stdout SHALL appear first under a "stdout" label
- **AND** stderr SHALL appear below under a "stderr" label with a red-tinted background

#### Scenario: Job with only stdout

- **WHEN** user views full output for a job with no stderr
- **THEN** only the stdout section SHALL be displayed
- **AND** no stderr section or header SHALL appear
