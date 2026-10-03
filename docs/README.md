# Molten documentation map

Choose the documentation by the question you need to answer. The two companion collections add 200 source-linked articles; they supplement the governing documents rather than replace them.

## Start with your task

| Question | Entry point |
| --- | --- |
| How do I contribute, inspect a workflow, or diagnose a failure? | [Workflow handbook](handbook/README.md): 100 walkthroughs, how-to guides, references, troubleshooting articles, and review checklists. |
| Why does a subsystem work this way, and where do its guarantees stop? | [Technical companion](technical/README.md): 100 implementation-focused articles on invariants, failure cases, and evidence boundaries. |
| What is the architecture and ownership model? | [Architecture](architecture.md), [distributed-system fabric](distributed-system-fabric.md), and [modularity boundaries](modularity-boundaries.md). |
| What evidence supports a change or operational claim? | [Proof workflow](proof-workflow.md), [distributed testing](distributed-testing.md), and [production runbooks](production-operator-runbooks.md). |

## Governing subsystem entry points

These links are a starting map, not a replacement inventory of requirements.

- **Authority and effects:** [nominal references](nominal-authority-references.md), [fabric port ownership](fabric-port-ownership.md), [node-state filesystem authority](node-state-filesystem-authority.md), and [effect profiles](effect-manifest-profiles.md).
- **Execution:** [system extensions](system-extension-runtime.md), [native extension host](native-system-extension-host.md), [Wasm components](wasm-component-runtime.md), and [bounded execution](fabric-execution.md).
- **Distributed mechanisms:** [transport sessions](fabric-transport-session-runtime.md), [membership and placement](fabric-membership-placement.md), [cryptographic identity](fabric-cryptographic-identity.md), and [time and scheduling](fabric-time-scheduler-runtime.md).
- **Data movement:** [content stores](content-store-adapter.md), [DAG sync](dag-sync.md), [content replication](content-replication.md), and [coordination delivery](coordination-delivery.md).
- **World lifecycle:** [commits](world-commit.md), [branch heads](world-branch-heads.md), [merge](world-state-diff-and-merge.md), [promotion](world-promotion-and-effect-release.md), and [operator workflows](world-operator-workflows.md).
- **Development and review:** [reproducible dependencies](reproducible-dependencies.md), [test workspaces](test-workspace-authority.md), [Nickel toolchain](nickel-toolchain.md), and [authority audits](ast-grep-runtime-authority-audits.md).

## How to interpret a document

Explanatory prose, source inspection, fixture observations, live observations, and canonical evidence establish different things. A command example is not a recorded execution. A fixture pass is not production readiness. A receipt's identity is not permission to act. The handbook and technical companion preserve these distinctions and link to the implementation behind specific claims.

Some articles identify unresolved differences between governing prose and inspected code. Read them as scoped review findings, not reproduced bug reports or newly approved exceptions. Consult the exact implementation and accepted requirements before relying on a disputed guarantee. The [technical companion's discrepancy pointers](technical/README.md#reading-source-discrepancies) collect several examples.

Accepted requirements live under [`.cairn/specs/`](../.cairn/specs/); active lifecycle work lives under [`.cairn/changes/`](../.cairn/changes/). Neither companion collection changes those requirements or supplies lifecycle approval.
