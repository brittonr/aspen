# Review: DAG fabric admission binding

The DAG owner exports one canonical descriptor; federation admits only that exact binding and routes the host-owned callback through `run_dag_sync` after checking the effect's request identity and generation. Wrong binding, stale generation, foreign request, authority denial, resource denial and corrupted content are each exercised (`evidence/2026-10-30-verification.md`).

The conformance reference used by the live rails is an executed, integration-owner-reviewed transcript recorded as explicitly experimental; it is not independent DAG owner approval and grants no production admission. Accepted for archive under that scope.
