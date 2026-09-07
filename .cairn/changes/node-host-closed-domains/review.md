# Domain and compatibility review

Single-agent technical review before implementation; not independent review or runtime approval.
Reviewed product source: `ab3efbe9788643caddd7f22bb0a13eca0d90426f`, retained on evidence branch `89800cbd5`.

## Decision

Proceed with the three explicitly scoped conditional declarations and their tests.
The domain contract is supported by the existing authority design, not by a target diagnostic count:

- LocalStoreKind selects eight fixed storage directory labels. A new kind requires an explicit mapping, not a default directory.
- NodeStateNamespaceKind selects 14 fixed namespace kinds. The registry order and two directory aliases are intentional; entry authority still compares exact kind, root, and scope.
- NodeStateFileObservation distinguishes missing leaves, non-regular denial, and a previously acquired regular handle. Its consumers must decide explicitly about any added observation category.

Sources reviewed: `docs/node-state-filesystem-authority.md`, `local_store/mod.rs`, `node/state/{authority,namespace,filesystem}.rs`, and existing `tests/{views,leaves}.rs` under `crates/molten-node-host`.
No enum is treated as an FSM. No public name, variant, payload, match arm, error, effect, or authority predicate needs to change.

## Compatibility decision

Accept conditional tool registration under the driver's existing `dylint_lib = "octet"` cfg, plus a specific Cargo expected-cfg declaration.
Do not lower warning severity. Ordinary compilation must not enable register_tool through these annotations.
Six retained reduced controls support this route, including stable compilation without RUSTC_BOOTSTRAP, active-driver admission, disabled-marker diagnostics, and E0004 on variant addition.
Actual package integration and actual-source mutation evidence are still required.

## Verification limits

Structural Cairn PASS is not approval of runtime behavior. Tests of fixed expected registries do not discover arbitrary future source edits automatically.
The remaining source gate, production compiler binding, approved cohort, and VM/replay work stay separate and blocked.
No new compiler/tool acquisition, Stage0, wildcard arms, blanket allows, or startup guard changes are authorized by this review.
