## ADDED Requirements

### Requirement: Merge button on patch detail page

The patch detail page SHALL display a merge button when the patch is open. The button SHALL be enabled when the patch is mergeable and disabled with a reason when it is not.

#### Scenario: Mergeable patch shows enabled merge button

- **GIVEN** patch P is open and mergeable (protection satisfied, no conflicts)
- **WHEN** the patch detail page is loaded
- **THEN** a merge button SHALL be displayed and enabled
- **AND** the button SHALL offer strategy selection (merge commit, squash)

#### Scenario: Blocked patch shows disabled merge button with reason

- **GIVEN** patch P is open but blocked by branch protection (missing approval)
- **WHEN** the patch detail page is loaded
- **THEN** a merge button SHALL be displayed but disabled
- **AND** the reason SHALL be shown (e.g., "Requires 1 more approval")

#### Scenario: Conflicting patch shows conflict indicator

- **GIVEN** patch P has merge conflicts
- **WHEN** the patch detail page is loaded
- **THEN** the merge button SHALL be disabled
- **AND** the conflicting file paths SHALL be listed

#### Scenario: Merged patch hides merge button

- **GIVEN** patch P is in `Merged` state
- **WHEN** the patch detail page is loaded
- **THEN** no merge button SHALL be displayed
- **AND** the merge commit hash SHALL be shown

### Requirement: Merge action via form POST

Clicking the merge button SHALL submit a form POST to the server. The server SHALL execute the `ForgeMergePatch` RPC and redirect back to the patch detail page.

#### Scenario: Successful merge redirects with success

- **WHEN** the user clicks merge with strategy "merge"
- **THEN** the server SHALL call `ForgeMergePatch` with the selected strategy
- **AND** redirect to the patch detail page
- **AND** the patch SHALL now show as Merged

#### Scenario: Failed merge shows error

- **WHEN** the merge fails (e.g., CAS exhausted, conflict appeared between check and merge)
- **THEN** the server SHALL redirect to the patch detail page with an error message

### Requirement: Approve button on patch detail page

The patch detail page SHALL display an approve button when the patch is open and the viewer has not already approved the current revision.

#### Scenario: Approve button visible for open patch

- **GIVEN** patch P is open
- **WHEN** the patch detail page is loaded
- **THEN** an approve button SHALL be displayed

#### Scenario: Approve action calls RPC

- **WHEN** the user clicks approve
- **THEN** the server SHALL call `ForgeApprovePatch` with the current patch head commit
- **AND** redirect to the patch detail page showing the new approval

### Requirement: Mergeability status banner

The patch detail page SHALL display a status banner showing protection check results, CI status, and conflict state.

#### Scenario: All checks passing

- **WHEN** CI is green, approvals are sufficient, no conflicts
- **THEN** a green banner SHALL indicate "Ready to merge"

#### Scenario: CI pending

- **WHEN** CI status is Pending
- **THEN** a yellow banner SHALL indicate "Checks in progress"

#### Scenario: Protection checks failing

- **WHEN** required approvals are not met
- **THEN** a red banner SHALL list the blocking conditions

### Requirement: Diff view on patch detail page

The patch detail page SHALL display the tree-level diff between the patch head and the target branch.

#### Scenario: Show file changes

- **GIVEN** patch P modifies `src/main.rs`, adds `README.md`, removes `old.txt`
- **WHEN** the patch detail page is loaded
- **THEN** the diff section SHALL list each file with its change type (added, modified, removed)

#### Scenario: Empty diff for identical trees

- **GIVEN** patch head tree matches target branch tree
- **WHEN** the patch detail page is loaded
- **THEN** the diff section SHALL indicate no changes
