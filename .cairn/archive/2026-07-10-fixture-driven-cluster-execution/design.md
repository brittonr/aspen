# Design: fixture-driven-cluster-execution

## Overview

Use the existing typed Nickel scenario fixture vocabulary to drive cluster and VM run planning. The fixture describes intent; the run gate compares observed artifacts against that intent before pass evidence is accepted.

## Functional core and shell boundary

The pure core accepts checked fixture metadata and observed run metadata. It validates that topology profile, execution profile, command surface, expected artifact kinds, required receipt refs, variance refs, unavailable policy, and caveats align.

The shell owns Nickel export/check commands, Nix VM execution, local cluster execution, file discovery, and writing observed artifacts.

## Execution plan derivation

A fixture-derived plan should include:

- scenario id and topology profile;
- execution profile and command surface;
- expected artifact kinds and required child receipt kinds;
- declared variance refs and diagnostic log refs;
- unavailable policy and evidence-only caveats;
- pass-claim eligibility.

## Observation gate

Observed run metadata must name the same scenario and command surface, include the expected artifact kinds, preserve required caveats, and deny unsupported pass claims or log-only success.

## Boundaries

Fixtures are review and planning inputs. They do not make a VM run pass; canonical child receipts and gates remain required.
