# Local filesystem authority boundary

Molten local artifact stores use capability roots for filesystem effects. The outer runtime or adapter shell opens an operator-declared root, then local store adapters operate through typed roots for artifact, chunk, retention, dataspace, and exchange state.

Capability roots bound local filesystem authority only. They do not prove artifact truth, confidentiality, remote transport trust, Merkle correctness, policy admission, deployment safety, or distributed runtime correctness. Those claims still require their existing manifests, content refs, receipts, policy evidence, provenance evidence, and runtime gates.

Local filesystem locators are accepted only as relative paths beneath the declared root. Parent traversal, absolute paths, content refs, remote tickets, URLs, and other transport locators are not local filesystem authority. Symlink escape attempts must fail before local artifact bytes outside the declared root are read, written, deleted, or exposed.
