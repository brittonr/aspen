# Validation evidence

## Goal

Publish Molten `0.1.0` only as a limited internal pilot for frozen source candidate `blake3:80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`.

## Completion Evidence

Completion requires passing fresh Rust, nextest, broad Nix, strict Cairn, executable VM, dogfood, signed bundle, promotion, export, release-profile, pilot-decision, and candidate-gate evidence. Octet evidence must use current configuration identity. A warning-only Octet deny receipt can support only the caveated pilot. Every artifact must bind the frozen candidate. Positive and denial receipts must remain available.

## False Completion Cases

Test counts, fixture checks, diagnostic logs, warning-only Octet output, skipped VM work, stale store paths, mixed-source receipts, and task checkboxes do not establish release readiness.

## Portfolio Search

| Family | Mechanism | State | Reason |
|---|---|---|---|
| Git-frame | BLAKE3 over the framed Git commit and tree | validated | Portable immutable source identity with explicit scope |
| Nix-path | Nix store source path | blocked | Binds one source filter and evaluator context, not the reviewed Git candidate |
| File-manifest | Sorted path and per-file BLAKE3 manifest | blocked | Needs a separately versioned canonicalization contract and tests |

The selected source frame is recorded in `design.md`. The passes are serial and correlated because subagent consent was not granted.

## Audit Risks

The audit checks dirty or changed source, mixed candidate bindings, missing evidence, fixture promotion, warning-only source gates, unsupported VM execution, stale readbacks, and unsupported production wording.

## Budget

Use one canonical source, three identity mechanisms, one positive and one negative candidate path, the smallest focused checks, and the required broad release rails. Terminal states are validated or blocked with an exact failing artifact.

## Results

### Candidate freeze

The initial candidate exposed feature-dependent Octet metadata hashing. That candidate was rejected before publication. The repair was validated, archived, committed as `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`, and integrated before the final freeze.

The final candidate is commit `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`, tree `58a6763c3668121ffa7309195f8d2c76ef4950d3`, and source ref `blake3:80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`. The clean detached worktree remained free of tracked changes.

### Rust, Nix, Cairn, and Octet

Formatting and workspace Clippy passed across all targets and features. The broad Nix rail passed 1,418 tests with no failures or skips. The CI receipt is `blake3:d59616502a20cee1d48447db4fbdf1d8fc2edcbc09b78838b408d50efa139044`.

Strict Cairn validation passed. The preserved JSON byte identity is `blake3:7e291f4b241552c4154c06d0b2946373764c03752c6701332726d2021bbfc85d`.

Pinned Octet revision `fc38f59330b626961d166febfdf1a5aa6575460f` produced current configuration and profile identities. Its status remains warning-only with 5,771 findings and no tool errors. Strict gate receipt `blake3:b6425a4eb1797cd2e60587056292fe75d675d7dc974f2852a292e88cd6aa8e07` denies that evidence. The pilot preserves and binds the deny receipt.

### VM and dogfood

The two-node NixOS VM check passed with test-run `blake3:d0a35c6c55f249a678b715437e75b89f798c0861ee3051e0a30c1c2bbcd82444`, soak `blake3:e7bd1bed7f67e4db5c38397c6083af2c987432f3119ee979cc82d6209b2c05f3`, and validation `blake3:f41c58d226085d33b0accbc8395f5ea0776be6ca97d76c3cea6e13c4ac65c78a`.

The VM network-control fault was unavailable. Receipt `blake3:a4575b0c807e98b25ab377328e55b5f2750380c7372507788040b1672213ca19` records no pass claim for that fault.

Dogfood passed with report `blake3:83cabd032ebee3f080c86bef40f4077b5556d0d78be76d70a92da4c841b4e407` and Nix verification `blake3:bb8a69e0647451bfb54c49082a4ddf905859ddad2c9cc45dc7c328cc941f9581`. Signed bundle verification passed as `blake3:833d35e693d20fc5ea34c60ba940e233eeb6a3eb3fb8770262c077f5b9a9d0a6`. Promotion passed as `blake3:74a49eb3674ac39c21d798b682a70aa48ba0cd972c87c39e435768c5c4346125`. Export verification passed as `blake3:5176eac8757539a7e75015d62844c79cff3c5c362ef3c925ccb405ef0db6dc0c`.

### Pilot decision and candidate gate

The production profile export is `blake3:b654547ce196b7c7c7a1a1415ed63dbb88456f95a66c0299dd5ea01a82d02a33`. Pilot-tier profile validation passed as `blake3:ddceda8c625403206b21bea149a82311c78573128d7600004d5823ad1fe658c7`. Release-tier validation remains unavailable because no reviewed Valence release-policy hash is selected.

The limited-pilot decision passed as `blake3:ea03ddbada910b33a09fe3156e9908b2b7c1f1b0c6b14b39570ae05abdaf27cb`. The candidate gate passed as `blake3:9d15e7f99a34ba5f1a979d07cd7023025347144349db50caacca6ddbb90ab1e4`.

A missing-candidate release profile denied as `blake3:83ff4aa76c74e588b12a7392d7f4930c8798f60fd5203dc18e86a856290e21ce`. A mixed-source candidate denied with the preserved bounded diagnostic.

The typed Nickel manifest exported successfully. Its mixed-source fixture failed its contract as required. The focused Nix check passed at `/nix/store/d9vxivmgqfccla3klw9vabfyy2hbxy7p-molten-release-pilot-manifest`.

The release-metadata branch also passed 1,418 tests with no failures or skips. Its metadata-branch receipt is `blake3:ccde865615b411b4a7ced2f9dc3104691e4d2ee31c92a161acc03c264acb49b0`. This receipt does not replace the frozen candidate receipt.

The release notes preserve pilot scope, rollback triggers, stop conditions, caveats, and production non-claims.

### Portfolio audit

The Git-frame mechanism survived the adversarial audit. The audit rejected the first candidate, found the Octet JSON-order defect, and confirmed the corrected evidence matrix. Nix-path and file-manifest identities remain blocked as source-identity alternatives for the reasons above.

The used budget was two candidate rounds, three source-identity mechanisms, one broad rail per accepted source state, and explicit positive and negative gates. The candidate evidence is validated for a limited internal pilot.

Strict Cairn validation and proposal, design, and task gates passed before sync. Initial gate receipts were proposal `95f652375408dcf0a81a2f80f7998ec04be9a18b4810d22da4c460997a11f154`, design `cf484684a0e1ce6f3e68681d953a094eec7080e0be8acc83201006c7317d1c80`, and tasks `42133fd310f48d43ec2f920b8aa3d4933e7e707807e776f24bd45be90fbc3393`.

Final pre-sync gate receipts were proposal `cc501fc034de806828acf2cfc96d1dfa7d89c08d7c718750f85f247345a110bf`, design `14c8f0118395029bbf5ab44624e7a303576f78dc53a948b33cbb5db109cba3c7`, and tasks `d4abfe77fc04d5ca74e9a7006517f8db156efb52790c6d0c4495d9abb4e59dee`. The sync dry-run plan was `fe8fb023371701625da1a972c7920bb14e7a835ecf9d8b1a5a8431fd7727ddc5`. Sync execution used plan `2434a6a5a15d500164be012f4457a4610f1d948bd57439a569bbb1b475ce5c81` and receipt `32d75f705c8673317d88c1c6a7fb7c3d8d86757b5dc9f99091785689a9b744a3`. Strict validation passed after sync. The archive dry-run plan was `bc264334e30692932e6b0841e3ba4fba38b4a6b7b2c8f53dbe6a41b9aaa7b0ec`. Archive execution used plan `7de5b0d66be369d33c1856598a369aa5f8a2dd398952da4223ceaeaa9712f657` and receipt `ffeb70ed29012cd10b10f39885ef35de52b4b405190f6cc73fdbac1f01c5bb74`. Strict validation passed after archive. The portfolio terminal state is validated for the named limited internal pilot. Remote publication remains pending.
