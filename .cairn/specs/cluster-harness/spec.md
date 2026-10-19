# Cluster Harness Specification

## Purpose

Defines the `cluster-harness` capability.

## Requirements

### Requirement: Harness fixtures use the existing cluster manifest byte admission before staging

r[molten.testing.cluster_harness_fixture_file_preflight] `execute_cluster_harness` MUST read an operator-supplied fixture through the existing cluster parser's `MAX_CLUSTER_MANIFEST_BYTES = 1,048,576` byte boundary with no more than one excess byte read for denial. It MUST complete fixture parsing and planning before deleting, creating or staging state and output directories or starting a child. It MUST preserve the existing 512-node admission, successful exactly-1-MiB fixture run, malformed fixture errors and the parser's +1 byte-denial shape. The harness MUST NOT create a separate byte cap or bypass the parser. The canonical isolated owner requires the earlier `aspen-octet-quality-owner-20260929` parser/CLI prerequisite for adoption; composition already contains it.

#### Scenario: Exact cluster manifest boundary executes

- GIVEN a valid fixture with one node and exactly 1,048,576 UTF-8 bytes
- WHEN the compiled `cluster harness-run` CLI executes the fixture
- THEN the parser admits it and the harness stages a run with a real child startup receipt.

#### Scenario: One extra byte denies before output directory effects

- GIVEN a valid-prefix fixture of 1,048,577 UTF-8 bytes and preexisting state/output directories with sentinel files
- WHEN the compiled `cluster harness-run --force` CLI attempts execution
- THEN the parser returns its existing `cluster manifest exceeds byte bound 1048576` denial after reading no more than limit plus one byte, both sentinels survive, no run artifact index is staged and no child state is created.

#### Scenario: Existing manifest shape errors remain parser-owned

- GIVEN an admitted-size fixture with an unsupported header or no nodes
- WHEN the harness plans the fixture
- THEN the parser returns its existing shape error before output-root mutation or child startup.
