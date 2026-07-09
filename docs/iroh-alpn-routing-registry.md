# Iroh ALPN routing registry

Molten-owned Iroh protocols are admitted through canonical ALPN registry entries before live router mutation. Each entry binds a symbolic name, ALPN string, owner namespace, handler profile, supported schema/profile versions, lifecycle state, limit refs, required evidence refs, receipt schema refs, and a canonical entry ref.

Router install, replacement, removal, and unsupported-ALPN receipts bind the registry entry ref when the ALPN is known. Admission checks ALPN formatting, owner namespace, handler profile, lifecycle state, generation, shutdown evidence, and explicit authority/policy/resource/evidence refs before mutating the live handler map.

ALPN routing remains transport evidence only. A valid ALPN, endpoint identity, router receipt, stream session, or framed envelope receipt does not grant node-control authority, policy admission, provenance, source-gate trust, resource authority, retention clearance, or execution permission.
