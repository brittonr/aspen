# Admit direct-LAN streams

Implement issues #15 and #7 through the existing cross-process Iroh 1.0 shell, protocol registration, endpoint descriptors and pure session transitions. Preserve the current loopback API. Add explicit direct endpoint configuration, actual authenticated peer binding, typed receive admission and bounded protected-socket streams. Actual two-host and Kiln/Cloud Hypervisor qualification require external targets and remain unexecuted; no source change can substitute for those observations.
