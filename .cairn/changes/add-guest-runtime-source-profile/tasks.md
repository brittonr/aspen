## Tasks

- [x] [serial] Add the distinct guest runtime producer profile and accept it without changing the full profile. r[molten.startup_binding.guest_profile]
- [x] [serial] Add a producer that measures the runtime cohort, builds an input-addressed output, and signs it with an operator key. r[molten.startup_binding.guest_producer]
- [ ] [depends:guest-runtime-producer] Run the signed guest output through the collaboration driver on the host and retain the baseline observation. r[molten.startup_binding.guest_admission]
- [ ] [depends:guest-runtime-admission] Run the signed guest profile in a KVM guest and retain bounded campaign evidence. r[molten.startup_binding.guest_campaign]
- [x] [depends:guest-runtime-producer] Add the exact-key guest admission cache and keep it unavailable to the full profile. r[molten.startup_binding.guest_admission_cache]
