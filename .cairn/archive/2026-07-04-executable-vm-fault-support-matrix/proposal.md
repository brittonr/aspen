## Why

The executable VM fault documentation correctly says unsupported host or VM fault support must produce unavailable or deny evidence, but the implementation path should make supported and unsupported cases explicit. Without a support matrix, reviewers cannot quickly tell which platform faults were actually injected and which were unavailable on the host.

## What Changes

- Add an executable VM fault support matrix for network control, process restart, filesystem permission, bounded disk pressure, receipt tamper, wrong topology, and log-only pass negatives.
- Bind host support status, preflight refs, injection refs, child workflow refs, post-fault refs, diagnostics, and caveats into validation evidence.
- Add supported-path checks where the VM image exposes the needed capability, and unavailable-path receipts where it does not.
- Add negative validation fixtures for unsupported pass claims, missing injection refs, missing diagnostic evidence, wrong topology, and log-only pass claims.

## Impact

VM fault evidence becomes easier to audit. A missing host feature no longer looks like a skipped success, and supported fault classes produce canonical evidence that can be compared across runs.