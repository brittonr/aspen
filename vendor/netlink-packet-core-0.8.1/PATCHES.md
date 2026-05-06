# Local Aspen patches

- Replaced unmaintained `paste` with maintained `pastey` while preserving the exported `paste!` macro surface used by downstream netlink macros.

Remove this vendor patch when upstream netlink-packet-core moves to a maintained token-pasting macro or Aspen upgrades to a parent release that no longer selects `paste`.
