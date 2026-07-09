## Tasks

- [x] [serial] Add `cap-std` only to Aspen crates/modules that own local store filesystem effects. r[molten.chunk_store.cap_std_boundary.dependency]
- [x] [serial] Introduce typed capability roots for artifact, chunk, retention, dataspace, and exchange local roots. r[molten.chunk_store.cap_std_boundary.root_wrappers]
- [x] [serial] Convert targeted local store opens to capability-relative operations while preserving manifest/catalog cores. r[molten.chunk_store.cap_std_boundary.conversion]
- [x] [serial] Add positive fixtures for valid chunks, manifests, and retention artifacts under declared roots. r[molten.chunk_store.cap_std_boundary.tests.positive]
- [x] [serial] Add negative fixtures for traversal, absolute paths, symlink escapes, missing roots, and remote-locator misuse. r[molten.chunk_store.cap_std_boundary.tests.negative]
- [x] [serial] Document the local filesystem-authority boundary and artifact-store non-claims. r[molten.chunk_store.cap_std_boundary.docs]
- [x] [serial] Run focused chunk-store/retention/dataspace checks plus Cairn validation and gates. r[molten.chunk_store.cap_std_boundary.validation]
