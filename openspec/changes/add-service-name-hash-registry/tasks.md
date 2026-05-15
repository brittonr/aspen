## Phase 1: Registry model

- [ ] [serial] Inventory runtime-service, deploy, dogfood, and CLI places that currently conflate service name, deployment ID, artifact ID, and receipt ID.
- [ ] [depends:inventory] Define `ServiceName`, `ServiceHash`, generation, target validation, and registry state model.

## Phase 2: Mutation and lookup behavior

- [ ] [depends:model] Implement Raft-backed assign/update/rollback semantics with authorization checks.
- [ ] [depends:mutation] Implement lookup/readback that always reports name, resolved hash, and generation or typed not-found.
- [ ] [depends:lookup] Add secret-safe receipts for assignment, update, rollback, and failed authorization.

## Phase 3: Validation

- [ ] [depends:receipts] Add positive tests for assign/update/rollback and negative tests for missing target, stale generation if applicable, and unauthorized mutation.
- [ ] [depends:tests] Update operator/developer docs and run focused registry tests, strict OpenSpec validation, and `git diff --check`.
