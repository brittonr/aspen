## Design

Aspen will open local artifact roots at the outer runtime or adapter shell and represent them with typed wrappers around `cap_std::fs::Dir`. Store code receives those wrappers rather than ambient `PathBuf` roots. Manifest and catalog logic continues to work with content refs, chunk refs, visibility policy, and in-memory DTOs.

The first conversion targets are `src/chunk/parts/store/*`, `src/retention/parts/*`, `src/remote/parts/dataspace/*`, and `src/iroh/parts/exchange/*`. Remote tickets, blob IDs, manifest refs, and Iroh routing metadata remain identity or transport hints, not filesystem authority.

Capability roots should replace parent traversal and absolute-path checks for local store operations. Negative tests must include `../` traversal, absolute path input, symlink escape attempts, missing root authority, and attempts to treat remote locators as local filesystem paths.

The change does not claim confidentiality, remote transport trust, artifact truth, Merkle correctness, or distributed runtime correctness. It only bounds local filesystem access beneath explicit roots.
