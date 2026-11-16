## Why

Issue #61: node live tickets and cross-process locators serialize IP hints with `SocketAddr`'s text form, which omits IPv6 `flowinfo` and carries a numeric `scope_id` that names an interface only on the producing host. Neither boundary stated which addresses it admits.

## What Changes

- One shared admission (`fabric_transport::ip_hint`) defines the exchanged-hint domain: unicast IPv4 and unscoped, flow-label-free unicast IPv6; unspecified, multicast, link-local, flow-labelled and scoped hints are outside it.
- Ticket and locator producers omit local addresses outside the domain; ticket and locator consumers reject them with the field named.
- The direct-LAN streaming document records the inventory, domain and compatibility assessment.

## Impact

- **Files**: the new hint module, the node live-ticket helpers, the cross-process locator conversion, and the streaming document.
- **Testing**: fixed cases for each address class, malformed scope text, a two-host interface-map fixture, and a property test over the production admission and text round trip.
- **Owner / consumer**: fabric transport. Cross-host qualification remains with #15.
