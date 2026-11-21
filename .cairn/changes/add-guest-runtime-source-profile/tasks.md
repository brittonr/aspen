## Tasks

- [x] [serial] Add the distinct guest runtime producer profile and accept it without changing the full profile. r[molten.startup_binding.guest_profile]
- [x] [serial] Add a producer that measures the runtime cohort, builds an input-addressed output, and signs it with an operator key. r[molten.startup_binding.guest_producer]
- [x] [depends:guest-runtime-producer] Run the signed guest output through the collaboration driver on the host and retain the baseline observation. r[molten.startup_binding.guest_admission]
- [ ] [depends:guest-runtime-admission] Run the signed guest profile in a KVM guest and retain bounded campaign evidence. r[molten.startup_binding.guest_campaign]
- [x] [serial] Verify that guest startup still denies an untrusted signer and a changed executable when old cache material is present. r[molten.startup_binding.guest_admission]

The [freshness and timer observations](evidence/2026-11-21-fresh-admission-and-timers.md) distinguish the new source-denial controls from the historical fast-cache result.
