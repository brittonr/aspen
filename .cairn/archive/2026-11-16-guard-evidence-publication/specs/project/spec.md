# Project Specification Delta

## ADDED Requirements

### Requirement: Published evidence excludes host-specific material
r[molten.project.evidence_publication.host_material] Files added or modified under `evidence/`, `release/`, `.cairn/changes/<change>/evidence/` and `.cairn/archive/<archive>/evidence/` MUST NOT contain absolute home paths, `/dev/shm/` run paths, `.local` or observed machine host names, RFC 1918 IPv4 addresses, CGNAT/tailnet (100.64.0.0/10) IPv4 addresses, or compressed containers whose members cannot be scanned; files under `docs/` MUST NOT contain home or `/dev/shm/` paths. The public GitHub-hosted runner home is not host-specific. A file published before this rule MAY keep its material only while its exact bytes match a baseline row binding its path to their BLAKE3; the baseline MUST NOT gain rows except identical bytes moved to a new path. A genuine example MAY be exempted only by a reviewed allowlist row naming the file, the exact matched text and a reason, or, under `docs/`, by an inline marker on the example's line. The pre-commit hook MUST check staged files and the repository check MUST check the whole tree, report each rejection with its file and line, fail on stale baseline rows, and fail closed on malformed or missing configuration. Historical evidence and archives MUST NOT be rewritten to satisfy this rule.

#### Scenario: New evidence with host material is rejected
- GIVEN a new evidence file containing a home path, a `.local` host, a 100.100.x.x address and a 192.168.x.x address
- WHEN the publication guard checks it
- THEN the guard fails and names the file, line and rule of each finding
- AND the same file with `<workspace>`, `<host>` and `<private-ip>` placeholders passes.

#### Scenario: A grandfathered file loses its exemption when edited
- GIVEN a historical evidence file whose path and BLAKE3 are in the baseline
- WHEN the guard checks the file before and after one byte changes
- THEN the unchanged file passes
- AND the edited file is rejected with its findings.

#### Scenario: Reviewed documentation examples pass
- GIVEN a documentation line carrying the inline marker, or an allowlist row naming the file and exact text
- WHEN the guard checks the file
- THEN those examples pass, while the same text in another file or without the marker is rejected.

### Requirement: Epoch-dated archives are limited to the recorded historical set
r[molten.project.evidence_publication.archive_dates] The repository check MUST reject any `.cairn/archive/1970-01-01-*` archive that is not in the recorded historical set, and any recorded historical archive that is missing, so that new archives carry their real date.

#### Scenario: A new epoch-dated archive fails the check
- GIVEN an archive named `1970-01-01-<change>` that is not in the recorded set
- WHEN the repository check runs
- THEN it fails and names the archive.
