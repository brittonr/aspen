## Why

Plugin host lifecycle is a state machine over install, permission, activation, hostcall, health, upgrade, and removal receipts. Host plugins are an authority and hostcall boundary, so lifecycle proofs must show that undeclared or unauthorized hostcalls and stale supply-chain evidence deny before side effects.

## What Changes

- Add requirements for plugin lifecycle and hostcall transition proof.
- Require proof traces that bind manifest, permission, lifecycle, hostcall, health, upgrade, and removal evidence.
- Require negative evidence for missing permission, wrong ABI, undeclared hostcall, failed health, stale supply-chain refs, and incomplete cleanup.

## Impact

- **Files**: plugin host core, hostcall admission, lifecycle receipt parsing, cleanup/upgrade logic, and plugin tests.
- **Testing**: valid install-to-removal path, denied unauthorized hostcall, failed health blocks upgrade/use, stale manifest denial, and cleanup receipt binding.
