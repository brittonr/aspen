# Design: Hermit/Uhyve runtime-host E2E promotion

## Context

The current Hermit row (`test-harness/suites/vm/runtime-host-hermit-gap.ncl`) is intentionally metadata-only. `aspen-runtime-core` already models `HermitUnikernelArtifact`, `HermitLaunchProfileKind::Uhyve`, `MicroVmEngine::Uhyve`, fail-closed admission, and secret-safe Hermit receipts. That model is not runtime-host proof.

The upstream Uhyve project (`https://github.com/hermit-os/uhyve`) documents `uhyve /path/to/the/unikernel/binary` as the Hermit-specific launch path. On Linux it depends on KVM. This change therefore treats Uhyve as the first Hermit execution candidate while preserving loader/QEMU as a fallback/future profile.

## Goals

- Launch a real Hermit unikernel artifact through Aspen-owned job/runtime orchestration.
- Keep Uhyve behind explicit capability/feature/gated test conditions.
- Produce bounded, secret-safe evidence with stable execution and guard markers.
- Preserve gap semantics until the gated proof actually executes.

## Non-Goals

- No production claim from model/admission coverage alone.
- No raw host command proof unless Aspen submitted/observed the job.
- No raw secrets, host-private paths, or mutable local image names in receipts.
- No claim that loader/QEMU or networked Hermit guests are ready.

## Decisions

### Uhyve is the first Hermit runner target

**Choice:** Implement the first Hermit product-path proof around Uhyve.

**Rationale:** Aspen already maps `HermitLaunchProfileKind::Uhyve` to `MicroVmEngine::Uhyve`, and Uhyve is the purpose-built Hermit hypervisor. It aligns with the user's likely prerequisite hint and avoids broad QEMU/loader scope.

**Alternative:** Start with loader/QEMU. Rejected for this slice because it adds loader artifact and boot-profile complexity before proving the direct Hermit/Uhyve path.

### Aspen orchestration owns evidence

**Choice:** The executable proof must submit a Hermit/Uhyve job through `JobManager`/`WorkerPool` or equivalent node worker registration.

**Rationale:** Previous row promotions established that direct worker-only calls, package builds, and shell smokes are insufficient. Hermit must meet the same product-path bar.

### Receipts distinguish execution from guardrails

**Choice:** Successful receipts must include `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`; negative/product-path guardrails must use `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD`.

**Rationale:** Stable markers prevent future docs/harness rows from accidentally treating admission checks or direct `uhyve` invocations as row promotion evidence.

## Implementation Sketch

1. Add a Hermit/Uhyve worker seam under the jobs/runtime-host execution surface, gated by a feature if necessary.
2. Define a job payload that references a blob-backed Hermit image hash, Uhyve runner capability, launch args, serial log limit, and timeout.
3. Execute `uhyve` via a bounded subprocess shell with explicit timeout, sanitized environment, bounded stdout/stderr capture, and no inherited secrets beyond a small allowlist.
4. Wrap completion in a structured receipt containing image hash, engine `Uhyve`, runner identity, lifecycle state, exit status, duration/bounded output, and proof marker.
5. Add a non-ignored guard test for malformed payloads/invalid image bytes that proves jobs reach the product worker before failing.
6. Add an ignored/gated proof test that requires real `uhyve` plus virtualization support and a declared Hermit fixture image, then promote the harness row only after this proof passes.

## Risks

- **Toolchain/build friction:** Hermit fixture builds may need the Hermit target/toolchain. Mitigation: allow `ASPEN_HERMIT_UHYVE_IMAGE` to point at a prebuilt fixture while keeping image hash/provenance in the receipt.
- **KVM availability:** Uhyve on Linux needs KVM. Mitigation: keep the proof gated/ignored unless the environment is capable.
- **Secret leakage:** Subprocess environments can leak host state. Mitigation: use an allowlisted environment and assert receipts exclude secret-like values.
- **Overclaiming:** A direct `uhyve` command can prove host tooling but not Aspen runtime orchestration. Mitigation: keep direct smokes as diagnostics only and require Aspen job state/receipt evidence for promotion.
