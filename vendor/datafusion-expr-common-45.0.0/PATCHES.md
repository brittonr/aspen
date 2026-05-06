# Local Aspen patches

- Replaced unmaintained `paste` with maintained `pastey` in interval arithmetic macro helpers.

Remove this vendor patch when upstream datafusion-expr-common moves to a maintained token-pasting macro or Aspen upgrades to a parent release that no longer selects `paste`.
