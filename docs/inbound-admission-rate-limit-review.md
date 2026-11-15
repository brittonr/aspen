# Inbound admission rate-limit review

Status: deterministic receive gate implemented for the explicit direct-LAN
stream shell; existing loopback transitions remain unchanged. Actual two-host
and Kiln consumer execution is still required for cross-host qualification.

## Admission boundary

The pure gate lives in
[`crates/molten-core/src/fabric_transport/receive.rs`](../crates/molten-core/src/fabric_transport/receive.rs).
The direct shell must preserve this order:

1. Capture the actual incoming IP address, not an endpoint descriptor's claimed
   locator. Reject non-IP transports in the direct-only profile.
2. Complete the authenticated Iroh handshake (not 0-RTT), then construct the
   bounded `ReceiveObservation` from `connection.remote_id()`, negotiated ALPN,
   and the exact service/context/generation/participant binding.
3. Call the listener-owned `ReceiveGate::admit_inbound` with injected monotonic
   ticks. Reject its typed deny before calling the protected service handler.
4. Pass the resulting **owned `GatedInbound`**, never a raw observation, to the
   protected authority handler. Only that handler may resolve membership,
   application-principal, trust-decision, and capability evidence and perform
   service admission. Gate success grants none of those authorities.
5. Only after authority admission may the shell deliver bounded stream data to
   the service. No automatic retry is introduced.

This gate budgets post-handshake Aspen admission attempts, not UDP parsing or
TLS handshakes. A source-level handshake flood therefore remains subject to
transport protections and deployment controls, not this application gate.

### Existing reference path is not a resolver

The prior review found no Aspen per-source rate limit. Its existing
[`PeerIdentityRefs::has_service_authority` and `validate_peer`](../crates/molten-core/src/fabric_transport/mod.rs)
check presence and canonical syntax of separate refs; they do not fetch or
verify the referenced membership, application principal, trust decision, or
capability. The legacy loopback
[`poll_one_derived_frame`](../src/fabric_transport/cross_process/iroh/parts/shell/p001/body.rs)
awaits a handshake, hashes the observed remote identity, and drives the existing
session transition using the descriptor's expected context. It is deliberately
not relabelled as a new protected authority-resolution implementation.

The direct-LAN protected handler is a distinct, typed boundary. Its owner must
supply substantive authority resolution; a successful gate or syntactically
valid reference is not a substitute. Session concurrency, credit, in-flight
bytes, queue slots, and frame length remain independent bounds.

## Stream terminal observations

The direct shell preserves native QUIC idle expiry as `Timeout`, including
stream opening, operation binding, frame/header/payload/ACK I/O and final
peer-stop observation. A peer reset, early EOF or application close remains
`Disconnected`; losing an acknowledgement after local consumption remains
uncertain regardless of the terminal class. No deadline or retry bound changes.

The existing `direct_stream_partial_header_obeys_idle_deadline` regression now
negotiates a shorter native QUIC idle deadline than the admitted application
deadline, forcing the transport-error path instead of depending on timer order.
The separate authenticated-disconnect control still requires `Disconnected`.
Repeat both through `cargo test --test directlanstreams` in the pinned
development environment; this same-host process/socket check does not supply
two-host or external consumer qualification.


## Deterministic budget

`ReceiveSource::from(IpAddr)` keys IPv4 by observed address and IPv6 by the high
64 bits. IPv4-mapped IPv6 normalizes to its IPv4 address. Ports, claimed peer
identity, context, service, generation, and role never partition the budget.
Private source-key construction prevents a caller from supplying an
unnormalized IPv6 host key.

`ReceiveBudget { burst, refill_ticks, max_sources }` requires positive limits
and a representable `burst * refill_ticks` horizon. Each attempt costs one token;
exactly one token refills per `refill_ticks`, capped at `burst`. Fractional refill
time is retained while below capacity; excess time is discarded on reaching
capacity. All arithmetic is integer-only. No clock, socket, DNS lookup, ref
resolution, or runtime is present in the core.

The `BTreeMap` contains at most `max_sources` entries. There is **no eviction**:
a new source at saturation gets `receive-source-saturated`, while admitted
sources retain their depleted budgets and can refill. This deliberately trades
new-source availability for fail-closed bounded memory and avoids churn-based
budget reset. A source-flood can exhaust the registry; owner/operator lifecycle
replacement is required to clear it. Recreating a gate after a denial is not a
retry mechanism. Keep one gate for the listener lifetime.

Ticks are globally nondecreasing, including across sources and denied attempts;
equal ticks are valid. Backwards ticks deny without changing state. A retry
whose future tick cannot be represented denies with `receive-tick-overflow`,
never a wrapped or saturated timestamp. Budget multiplication overflow denies
at construction. Very large elapsed intervals refill by quotient and capacity
comparison, without overflowing an intermediate token sum or tick product.

Observation validation is cheap and bounded: graphic ASCII peer identity and
ALPN (nonempty, at most 256 bytes each), a canonical BLAKE3 context ref, a bounded
service token, and positive generation. These checks assert shape only. Even
malformed observations consume a token, so malformed traffic cannot receive
unmetered validation. Text is moved into the gated result, not cloned.

Denials expose a typed `ReceiveDenyReason`, a stable `code()`, and relative
`retry_ticks: Option<u64>`. Only `receive-rate-limited` provides a delay;
invalid budgets/facts, registry saturation, backwards time, and overflow require
correction rather than a promised time-based retry. The shell owns any canonical
deny receipt; the core does not hash or render one.

`GatedInbound` has private fields, no public constructor, no deserialization, and
no `Clone`. `observation()` borrows the unchanged facts. Compile-fail doctests
cover forging it and passing a raw observation into a handler that requires it.
Inline tests cover source/identity isolation, IPv6 /64 grouping and mapped-v4
normalization, registry saturation/no eviction, fractional refill and burst
capping, backwards time, overflow, large tick jumps, malformed-input charging,
and unchanged in-budget observations. These authored tests are not execution
evidence until the main verification owner runs them.

## Selected Iroh/QUIC source review

The selected dependency path is the root `iroh = "1.0.0"` manifest requirement
through the crates.io entry in [`Cargo.lock`](../Cargo.lock), **not** the old
`vendor/iroh/Cargo.toml.orig`. The lock selects:

| Crate | Version | Registry checksum |
| --- | --- | --- |
| iroh | 1.0.0 | `6435544bb3a5c4e6ff7affaa0c0aa0d1bca45bd700226329d5059d3eb54f9dff` |
| noq | 1.0.0 | `11f0c73794bfde94db01379c46990b9a773993fca2b61a66184ce148b7c7a187` |
| noq-proto | 1.0.0 | `775be06b8d66c2c64db60140bf54dee8410f67b73c81cc1e1e32f11dfdaae501` |
| noq-udp | 1.0.0 | `cd5a37756f168cf350d68a97c4f0158bdf3c76f10175123941569b09ab51f011` |

Iroh 1.0.0 uses NOQ for this path; the separate Quinn packages in the workspace
lock are not the selected Iroh transport implementation. Registry package
`.cargo_vcs_info.json` maps Iroh to commit
`6520cd6c9bacea15823c53a38267c40d754980fe` (`iroh/`) and noq-proto to
`88b05460ce23985aa34f271c03f1b6c9db29a909` (`noq-proto/`). The following
permalinks bind the reviewed source, rather than moving branch documentation:

- [Iroh Incoming address, refusal, ignore, retry, and address-validation API](https://github.com/n0-computer/iroh/blob/6520cd6c9bacea15823c53a38267c40d754980fe/iroh/src/endpoint/connection.rs#L129-L215).
  `retry()` asks the client to retry with address validation; availability of this
  API does not establish that every caller enables retries or an application
  admission rate limit.
- [Authenticated remote identity and negotiated ALPN after handshake](https://github.com/n0-computer/iroh/blob/6520cd6c9bacea15823c53a38267c40d754980fe/iroh/src/endpoint/connection.rs#L1064-L1080).
  Remote identity comes from the TLS certificate, not the endpoint handoff.
- [Iroh pending-incoming and byte-buffer configuration](https://github.com/n0-computer/iroh/blob/6520cd6c9bacea15823c53a38267c40d754980fe/iroh/src/endpoint/quic.rs#L650-L703).
- [NOQ default pending/byte limits](https://github.com/n0-computer/noq/blob/88b05460ce23985aa34f271c03f1b6c9db29a909/noq-proto/src/config/mod.rs#L234-L259):
  65,536 pending incoming attempts, 10 MiB per incoming buffer, and 100 MiB
  collectively. These are upstream defaults, not proof of Aspen-specific
  configured limits or a per-source rate.
- [NOQ checked buffer bounds](https://github.com/n0-computer/noq/blob/88b05460ce23985aa34f271c03f1b6c9db29a909/noq-proto/src/endpoint.rs#L221-L237)
  and [pending-attempt refusal](https://github.com/n0-computer/noq/blob/88b05460ce23985aa34f271c03f1b6c9db29a909/noq-proto/src/endpoint.rs#L698-L706).
- [NOQ anti-amplification budget](https://github.com/n0-computer/noq/blob/88b05460ce23985aa34f271c03f1b6c9db29a909/noq-proto/src/connection/paths.rs#L420-L426)
  limits sending on an unvalidated path relative to received bytes. It does not
  meter repeated application authority resolution after authenticated connects.

These protections complement the Aspen receive gate; none is evidence of
membership, service permission, four-ref resolution, or cross-host application
qualification. Source inspection is not a network load-test result.

