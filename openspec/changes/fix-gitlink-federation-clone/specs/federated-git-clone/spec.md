## MODIFIED Requirements

### Requirement: Federation clone supports repos with submodules

Federation clone MUST succeed for repositories containing gitlink (submodule) tree entries. Previously, gitlinks were dropped during import causing tree SHA-1 mismatch on export.

#### Scenario: Federated clone of repo with submodules

- **WHEN** a federated clone is performed on a repository containing trees with mode 160000 entries
- **THEN** the clone completes successfully with all objects transferred and git reports no missing or bad objects

#### Scenario: Round-trip integrity for submodule trees

- **WHEN** a tree containing gitlinks is pushed to alice's forge, then fetched via federation clone through bob
- **THEN** the fetched tree has identical content and SHA-1 to the original
