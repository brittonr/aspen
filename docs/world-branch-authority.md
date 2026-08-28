# World branch authority

Molten treats branch authority as current runtime state. A world commit can store public observation identities. It cannot store or copy a live capability.

## Ownership

Basalt revision `89675cd4f585f837323c049e4a25f7b94c903038` owns the portable policy classes and obligations.

Molten owns these actions:

- token parsing and verification;
- current revocation and replay observations;
- scope normalization;
- capability derivation;
- durable linear ownership;
- simulation adapter selection;
- activation and effect execution;
- receipts and release decisions.

A Basalt decision does not mint, move, activate, or enforce a capability.

## Closed modes

`copyable` requires an independently current destination grant.

`attenuated` requires a destination scope that is strictly narrower than the source scope.

`linear` requires one generation-fenced transfer. The source must be inactive before destination activation.

`simulation-only` requires an exact deterministic simulation adapter. Molten never falls back to a live adapter.

`promotion-gated` requires a current promotion recheck and a release reservation. This path remains denied until the effect-release dependency closes.

`replace-before-activation` requires a new current destination grant.

`non-branchable` always denies activation.

Unknown or lossy mappings deny.

## Functional core

`molten-core::world_branch_authority` maps normalized product facts into the Basalt request.
It validates the policy decision and returns a product-owned obligation plan.

The core also validates supplied realization observations before activation.
It performs no file, network, clock, token, secret, adapter, or persistence operation.

## Imperative shell

The shell observes current policy and authority through application ports.
It realizes copy, attenuation, replacement, simulation, and linear obligations through separate capabilities.

Linear transfer uses one operation identity and one expected generation.
An unknown transfer outcome triggers observation-first reconciliation.
Molten does not retry the mutation blindly.

The shell rechecks policy and authority after realization and before activation.
It also rechecks durable ownership after a linear transfer.

An allowed activation decision is still not proof that the activation effect succeeded.
An unknown activation outcome triggers one observation-first reconciliation call, not another activation attempt.
The shell writes a separate outcome receipt, and unresolved outcomes remain unknown.

## Operator commands

`molten world-authority` provides `plan`, `authority-inspect`, `activate`, `transfer`, `simulate`, and `recover` commands.

The plan and inspection commands consume bounded public request JSON plus the reviewed policy file.
They print only metadata identities and can write a metadata-only plan receipt.

The effect commands require an explicit receipt path.
Without an admitted runtime adapter, they write a denial receipt and fail closed.
They never treat a policy receipt as live authority.

## Evidence and confidentiality

Plan, activation-admission, and activation-outcome receipts use canonical Preserves and contain metadata identities only.
They exclude bearer tokens, private keys, credentials, raw capability paths, private policy bodies, and secret entropy.

Receipts state these limits:

- policy decisions do not mint or move capabilities;
- receipts are not current authority;
- linear plans do not prove exclusive ownership;
- simulation plans do not prove parity or host confinement;
- promotion plans do not authorize dispatch;
- activation observations do not prove future enforcement;
- evidence does not prove release eligibility.

## Current blocker

The promotion-gated path depends on `bind-world-promotion-to-effect-release`.
That change remains blocked until Weft publishes the required replay and effect-log runtime.
Molten therefore plans the obligation but does not reserve or dispatch effects from this branch-authority work.
