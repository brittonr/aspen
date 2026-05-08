## Why

Aspen now has runtime-host proof rows, job/plugin execution paths, deploy concepts, and service-core modeling, but operators still lack one canonical runtime-service contract tying services, jobs/plugins, deploy actions, receipts, and route ownership together. A focused OpenSpec should define the next implementation seam before code spreads across ad hoc surfaces.

## What Changes

- Add canonical runtime-service contract requirements over service identity, host-loading reference, execution backend, deploy action, route ownership, health, and receipt identity.
- Require anti-overclaiming boundaries between model validation, admission, activation, and healthy runtime state.
- Establish secret-safe receipt links across service/job/plugin/deploy events.

## Capabilities

### Modified Capabilities
- `runtime-service-core`: Extends the service model with a canonical operator contract that can guide future implementation.

## Impact

- **Files**: runtime-service core model, job/plugin/deploy receipt adapters, docs/tests in future implementation.
- **APIs**: Future typed DTOs and adapters may be introduced; this change is spec-only.
- **Testing**: OpenSpec validation now; future model/serialization/receipt tests during implementation.
