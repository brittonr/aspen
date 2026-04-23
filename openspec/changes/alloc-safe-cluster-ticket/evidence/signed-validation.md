Evidence-ID: alloc-safe-cluster-ticket.v1-signed-validation
Task-ID: V3
Artifact-Type: command-transcript
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-ticket-support-requires-explicit-opt-in, architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-only-surface-stays-distinct-from-std-conveniences, ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug, ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.invalid-signed-cluster-ticket-string-is-still-rejected

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-ticket --no-default-features --features signed -e normal'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   │   ├── serde_core v1.0.228
│   │   └── serde_derive v1.0.228 (proc-macro)
│   │       ├── proc-macro2 v1.0.106
│   │       │   └── unicode-ident v1.0.24
│   │       ├── quote v1.0.45
│   │       │   └── proc-macro2 v1.0.106 (*)
│   │       └── syn v2.0.117
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── unicode-ident v1.0.24
│   └── thiserror v2.0.18
│       └── thiserror-impl v2.0.18 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quote v1.0.45 (*)
│           └── syn v2.0.117 (*)
├── iroh-base v0.97.0
│   ├── curve25519-dalek v5.0.0-pre.1
│   │   ├── cfg-if v1.0.4
│   │   ├── cpufeatures v0.2.17
│   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   ├── digest v0.11.0-rc.10
│   │   │   ├── block-buffer v0.11.0
│   │   │   │   └── hybrid-array v0.4.8
│   │   │   │       └── typenum v1.19.0
│   │   │   ├── const-oid v0.10.2
│   │   │   └── crypto-common v0.2.1
│   │   │       └── hybrid-array v0.4.8 (*)
│   │   ├── rand_core v0.9.5
│   │   ├── serde v1.0.228 (*)
│   │   ├── subtle v2.6.1
│   │   └── zeroize v1.8.2
│   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── syn v2.0.117 (*)
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │       ├── convert_case v0.10.0
│   │       │   └── unicode-segmentation v1.12.0
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       ├── syn v2.0.117 (*)
│   │       └── unicode-xid v0.2.6
│   ├── digest v0.11.0-rc.10 (*)
│   ├── ed25519-dalek v3.0.0-pre.1
│   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│   │   ├── ed25519 v3.0.0-rc.4
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── signature v3.0.0-rc.10
│   │   ├── rand_core v0.9.5
│   │   ├── serde v1.0.228 (*)
│   │   ├── sha2 v0.11.0-rc.2
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   └── digest v0.11.0-rc.10 (*)
│   │   ├── subtle v2.6.1
│   │   └── zeroize v1.8.2 (*)
│   ├── n0-error v0.1.3
│   │   ├── n0-error-macros v0.1.3 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   └── spez v0.1.2 (proc-macro)
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       └── syn v2.0.117 (*)
│   ├── rand_core v0.9.5
│   ├── serde v1.0.228 (*)
│   ├── sha2 v0.11.0-rc.2 (*)
│   ├── url v2.5.8
│   │   ├── form_urlencoded v1.2.2
│   │   │   └── percent-encoding v2.3.2
│   │   ├── idna v1.1.0
│   │   │   ├── idna_adapter v1.2.1
│   │   │   │   ├── icu_normalizer v2.1.1
│   │   │   │   │   ├── icu_collections v2.1.1
│   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro)
│   │   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   │   ├── potential_utf v0.1.4
│   │   │   │   │   │   │   └── zerovec v0.11.5
│   │   │   │   │   │   │       ├── yoke v0.8.1
│   │   │   │   │   │   │       │   ├── stable_deref_trait v1.2.1
│   │   │   │   │   │   │       │   ├── yoke-derive v0.8.1 (proc-macro)
│   │   │   │   │   │   │       │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │   │   ├── syn v2.0.117 (*)
│   │   │   │   │   │   │       │   │   └── synstructure v0.13.2
│   │   │   │   │   │   │       │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │   │       ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │   │       └── syn v2.0.117 (*)
│   │   │   │   │   │   │       │   └── zerofrom v0.1.6
│   │   │   │   │   │   │       │       └── zerofrom-derive v0.1.6 (proc-macro)
│   │   │   │   │   │   │       │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │           ├── syn v2.0.117 (*)
│   │   │   │   │   │   │       │           └── synstructure v0.13.2 (*)
│   │   │   │   │   │   │       ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │       └── zerovec-derive v0.11.2 (proc-macro)
│   │   │   │   │   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │           └── syn v2.0.117 (*)
│   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   ├── icu_normalizer_data v2.1.1
│   │   │   │   │   ├── icu_provider v2.1.1
│   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   ├── icu_locale_core v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── litemap v0.8.1
│   │   │   │   │   │   │   ├── tinystr v0.8.2
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   ├── zerotrie v0.2.3
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   └── zerofrom v0.1.6 (*)
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   ├── smallvec v1.15.1
│   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   └── icu_properties v2.1.2
│   │   │   │       ├── icu_collections v2.1.1 (*)
│   │   │   │       ├── icu_locale_core v2.1.1 (*)
│   │   │   │       ├── icu_properties_data v2.1.2
│   │   │   │       ├── icu_provider v2.1.1 (*)
│   │   │   │       ├── zerotrie v0.2.3 (*)
│   │   │   │       └── zerovec v0.11.5 (*)
│   │   │   ├── smallvec v1.15.1
│   │   │   └── utf8_iter v1.0.4
│   │   ├── percent-encoding v2.3.2
│   │   ├── serde v1.0.228 (*)
│   │   └── serde_derive v1.0.228 (proc-macro) (*)
│   ├── zeroize v1.8.2 (*)
│   └── zeroize_derive v1.4.3 (proc-macro) (*)
├── iroh-tickets v0.4.0
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1 (*)
│   ├── iroh-base v0.97.0 (*)
│   ├── n0-error v0.1.3 (*)
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── heapless v0.7.17
│   │   │   ├── hash32 v0.2.1
│   │   │   │   └── byteorder v1.5.0
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── spin v0.9.8
│   │   │   │   └── lock_api v0.4.14
│   │   │   │       └── scopeguard v1.2.0
│   │   │   └── stable_deref_trait v1.2.1
│   │   └── serde v1.0.228 (*)
│   └── serde v1.0.228 (*)
├── postcard v1.1.3 (*)
├── serde v1.0.228 (*)
└── thiserror v2.0.18 (*)

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-ticket --features std -e normal'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   │   ├── serde_core v1.0.228
│   │   └── serde_derive v1.0.228 (proc-macro)
│   │       ├── proc-macro2 v1.0.106
│   │       │   └── unicode-ident v1.0.24
│   │       ├── quote v1.0.45
│   │       │   └── proc-macro2 v1.0.106 (*)
│   │       └── syn v2.0.117
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── unicode-ident v1.0.24
│   └── thiserror v2.0.18
│       └── thiserror-impl v2.0.18 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quote v1.0.45 (*)
│           └── syn v2.0.117 (*)
├── iroh-base v0.97.0
│   ├── curve25519-dalek v5.0.0-pre.1
│   │   ├── cfg-if v1.0.4
│   │   ├── cpufeatures v0.2.17
│   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   ├── digest v0.11.0-rc.10
│   │   │   ├── block-buffer v0.11.0
│   │   │   │   └── hybrid-array v0.4.8
│   │   │   │       └── typenum v1.19.0
│   │   │   ├── const-oid v0.10.2
│   │   │   └── crypto-common v0.2.1
│   │   │       └── hybrid-array v0.4.8 (*)
│   │   ├── rand_core v0.9.5
│   │   │   └── getrandom v0.3.4
│   │   │       ├── cfg-if v1.0.4
│   │   │       └── libc v0.2.183
│   │   ├── serde v1.0.228 (*)
│   │   ├── subtle v2.6.1
│   │   └── zeroize v1.8.2
│   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── syn v2.0.117 (*)
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │       ├── convert_case v0.10.0
│   │       │   └── unicode-segmentation v1.12.0
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       ├── syn v2.0.117 (*)
│   │       └── unicode-xid v0.2.6
│   ├── digest v0.11.0-rc.10 (*)
│   ├── ed25519-dalek v3.0.0-pre.1
│   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│   │   ├── ed25519 v3.0.0-rc.4
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── signature v3.0.0-rc.10
│   │   ├── rand_core v0.9.5 (*)
│   │   ├── serde v1.0.228 (*)
│   │   ├── sha2 v0.11.0-rc.2
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   └── digest v0.11.0-rc.10 (*)
│   │   ├── subtle v2.6.1
│   │   └── zeroize v1.8.2 (*)
│   ├── n0-error v0.1.3
│   │   ├── n0-error-macros v0.1.3 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   └── spez v0.1.2 (proc-macro)
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       └── syn v2.0.117 (*)
│   ├── rand_core v0.9.5 (*)
│   ├── serde v1.0.228 (*)
│   ├── sha2 v0.11.0-rc.2 (*)
│   ├── url v2.5.8
│   │   ├── form_urlencoded v1.2.2
│   │   │   └── percent-encoding v2.3.2
│   │   ├── idna v1.1.0
│   │   │   ├── idna_adapter v1.2.1
│   │   │   │   ├── icu_normalizer v2.1.1
│   │   │   │   │   ├── icu_collections v2.1.1
│   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro)
│   │   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   │   ├── potential_utf v0.1.4
│   │   │   │   │   │   │   └── zerovec v0.11.5
│   │   │   │   │   │   │       ├── yoke v0.8.1
│   │   │   │   │   │   │       │   ├── stable_deref_trait v1.2.1
│   │   │   │   │   │   │       │   ├── yoke-derive v0.8.1 (proc-macro)
│   │   │   │   │   │   │       │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │   │   ├── syn v2.0.117 (*)
│   │   │   │   │   │   │       │   │   └── synstructure v0.13.2
│   │   │   │   │   │   │       │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │   │       ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │   │       └── syn v2.0.117 (*)
│   │   │   │   │   │   │       │   └── zerofrom v0.1.6
│   │   │   │   │   │   │       │       └── zerofrom-derive v0.1.6 (proc-macro)
│   │   │   │   │   │   │       │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │       │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │       │           ├── syn v2.0.117 (*)
│   │   │   │   │   │   │       │           └── synstructure v0.13.2 (*)
│   │   │   │   │   │   │       ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │       └── zerovec-derive v0.11.2 (proc-macro)
│   │   │   │   │   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │           └── syn v2.0.117 (*)
│   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   ├── icu_normalizer_data v2.1.1
│   │   │   │   │   ├── icu_provider v2.1.1
│   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   ├── icu_locale_core v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── litemap v0.8.1
│   │   │   │   │   │   │   ├── tinystr v0.8.2
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   ├── zerotrie v0.2.3
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   └── zerofrom v0.1.6 (*)
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   ├── smallvec v1.15.1
│   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   └── icu_properties v2.1.2
│   │   │   │       ├── icu_collections v2.1.1 (*)
│   │   │   │       ├── icu_locale_core v2.1.1 (*)
│   │   │   │       ├── icu_properties_data v2.1.2
│   │   │   │       ├── icu_provider v2.1.1 (*)
│   │   │   │       ├── zerotrie v0.2.3 (*)
│   │   │   │       └── zerovec v0.11.5 (*)
│   │   │   ├── smallvec v1.15.1
│   │   │   └── utf8_iter v1.0.4
│   │   ├── percent-encoding v2.3.2
│   │   ├── serde v1.0.228 (*)
│   │   └── serde_derive v1.0.228 (proc-macro) (*)
│   ├── zeroize v1.8.2 (*)
│   └── zeroize_derive v1.4.3 (proc-macro) (*)
├── iroh-tickets v0.4.0
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1 (*)
│   ├── iroh-base v0.97.0 (*)
│   ├── n0-error v0.1.3 (*)
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── heapless v0.7.17
│   │   │   ├── hash32 v0.2.1
│   │   │   │   └── byteorder v1.5.0
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── spin v0.9.8
│   │   │   │   └── lock_api v0.4.14
│   │   │   │       └── scopeguard v1.2.0
│   │   │   └── stable_deref_trait v1.2.1
│   │   └── serde v1.0.228 (*)
│   └── serde v1.0.228 (*)
├── postcard v1.1.3 (*)
├── rand v0.9.2
│   ├── rand_chacha v0.9.0
│   │   ├── ppv-lite86 v0.2.21
│   │   │   └── zerocopy v0.8.42
│   │   └── rand_core v0.9.5 (*)
│   └── rand_core v0.9.5 (*)
├── serde v1.0.228 (*)
└── thiserror v2.0.18 (*)

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ticket --no-default-features --features signed'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ticket --features std'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --no-default-features --features signed --test signed'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.30s
     Running tests/signed.rs (target/debug/deps/signed-681dd1d6ac4530c8)

running 3 tests
test signed_only_deserialize_rejects_corrupted_input ... ok
test signed_only_helpers_reject_expired_ticket ... ok
test signed_only_helpers_verify_at_explicit_time ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --no-default-features --features signed --test ui'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.30s
     Running tests/ui.rs (target/debug/deps/ui-934f39dd9e28a35d)

running 2 tests
warning: patch `snix-glue v0.1.0 (/home/brittonr/git/aspen/vendor/snix-glue)` was not used in the crate graph
warning: patch `cargo-hyperlight v0.1.5 (/home/brittonr/git/aspen/vendor/cargo-hyperlight)` was not used in the crate graph
warning: patch `uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)` was not used in the crate graph
help: Check that the patched package version and available features are compatible
      with the dependency requirements. If the patch has a different version from
      what is locked in the Cargo.lock file, run `cargo update` to use the new
      version. This may also occur with an optional dependency that is not enabled.
    Checking aspen-ticket-tests v0.0.0 (/home/brittonr/git/aspen/crates/aspen-ticket/target/tests/trybuild/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s


test [0m[1mtests/ui/iroh_helpers_require_feature.rs[0m ... [0m[32mok
[0m

test iroh_helpers_require_feature ... ok
warning: patch `snix-glue v0.1.0 (/home/brittonr/git/aspen/vendor/snix-glue)` was not used in the crate graph
warning: patch `cargo-hyperlight v0.1.5 (/home/brittonr/git/aspen/vendor/cargo-hyperlight)` was not used in the crate graph
warning: patch `uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)` was not used in the crate graph
help: Check that the patched package version and available features are compatible
      with the dependency requirements. If the patch has a different version from
      what is locked in the Cargo.lock file, run `cargo update` to use the new
      version. This may also occur with an optional dependency that is not enabled.
    Checking aspen-ticket-tests v0.0.0 (/home/brittonr/git/aspen/crates/aspen-ticket/target/tests/trybuild/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s


test [0m[1mtests/ui/std_wrappers_require_feature.rs[0m ... [0m[32mok
[0m

test std_wrappers_require_feature ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.38s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --features std --test std'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.34s
     Running tests/std.rs (target/debug/deps/std-e762c76514830cf4)

running 2 tests
test std_sign_with_validity_uses_current_time_wrappers ... ok
test std_signed_wrappers_work ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


## Deterministic negative assertions

Confirmed by the saved signed-only and std-only trees above: the signed-only surface excludes `rand`, `iroh`, `iroh-gossip`, and `anyhow`, and the `std` surface still excludes `iroh` and `iroh-gossip`.

## Deterministic source audit

### `python3 - <<\PY\ ... SignedAspenClusterTicket::to_bytes source audit ... PY`

source audit ok: to_bytes uses expect(...) with contextual diagnostics and no empty-payload fallback
