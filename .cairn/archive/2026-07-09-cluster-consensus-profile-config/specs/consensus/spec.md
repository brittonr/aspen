## ADDED Requirements

### Requirement: Cluster config selects consensus profile
r[molten.consensus.cluster_config_selection] Molten MUST allow cluster configuration to select the control-plane consensus algorithm profile and admitted profile version before group manifest construction. Omitted selection MUST preserve the current Raft production profile default. Selected profiles MUST still pass manifest validation and consensus engine registry admission before runtime construction, and configuration MUST NOT promote experimental, disabled, unknown, or evidence-incomplete engines into production by itself.

#### Scenario: Configured Raft profile starts runtime
- GIVEN cluster config selects the admitted Raft consensus profile and profile version
- WHEN Molten builds the control-plane group manifest from that config
- THEN the manifest records the selected profile and version
- AND runtime construction resolves the matching production-admitted engine through the registry.

#### Scenario: Experimental profile is manifestable but denied for production
- GIVEN cluster config selects an experimental leaderless consensus profile with required manifest refs
- WHEN Molten builds the control-plane group manifest from that config
- THEN the manifest records the selected experimental profile
- AND production runtime construction denies the profile unless separate admission evidence and policy promote it.

#### Scenario: Unknown configured profile is rejected
- GIVEN cluster config names an unknown or misspelled consensus profile
- WHEN Molten validates the config or builds the group manifest
- THEN validation fails before runtime construction
- AND diagnostics identify the unsupported configured profile without falling back to Raft.
