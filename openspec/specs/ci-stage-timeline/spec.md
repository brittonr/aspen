## ADDED Requirements

### Requirement: Stage progress bar on pipeline detail page

The pipeline detail page SHALL display a horizontal stage progress bar above the stage detail sections. The bar SHALL contain one segment per stage, each colored according to stage status: green for succeeded, red for failed, yellow for running, gray for pending, dark gray for cancelled. Each segment SHALL display the stage name and duration (if started).

#### Scenario: Pipeline with all stages succeeded

- **WHEN** user views a completed pipeline where all three stages (checkout, build, test) succeeded
- **THEN** the progress bar SHALL show three green segments, each labeled with the stage name and total duration

#### Scenario: Pipeline with a running stage

- **WHEN** user views a running pipeline where "checkout" succeeded and "build" is running
- **THEN** the progress bar SHALL show "checkout" in green with its duration, "build" in yellow with "running…", and subsequent stages in gray

#### Scenario: Pipeline with a failed stage

- **WHEN** user views a failed pipeline where "build" failed
- **THEN** the "build" segment SHALL be red, preceding stages green, and subsequent stages gray (never reached)

#### Scenario: Pipeline with single stage

- **WHEN** user views a pipeline with only one stage
- **THEN** the progress bar SHALL show a single full-width segment with the stage status color

### Requirement: Stage segments are equal width

Each stage segment in the progress bar SHALL have equal width regardless of its duration. This prevents short stages from becoming invisible next to long-running ones.

#### Scenario: Stages with very different durations

- **WHEN** a pipeline has a 5-second checkout stage and a 30-minute build stage
- **THEN** both segments SHALL occupy equal horizontal space in the progress bar
