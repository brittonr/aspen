# World fault focused Octet gate

This workspace compiles the pure world fault conformance core with the pinned strict Octet catalog.

The Nix check copies the production module into this workspace. It replaces only the product test module with an empty focused-test file.

This gate does not replace the workspace test, Clippy, Nix, restart, or Cairn rails.
