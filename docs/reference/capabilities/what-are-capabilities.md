# What Are Capabilities?

- **Author**: Chip Morningstar
- **Date**: May 7, 2017
- **URL**: https://habitatchronicles.com/2017/05/what-are-capabilities/

Accessible introduction to object capabilities (ocaps) from one of the creators of Habitat. Covers:

- ACL vs capability security models and why ACLs are "fatally flawed"
- The Confused Deputy problem (Norm Hardy's original story)
- "Don't separate designation from authority"
- Capabilities as unforgeable references that combine designation + authority
- Three ways to obtain a capability: creation, transfer, endowment
- Capability patterns: modulation (revokers), attenuation (least authority), abstraction, combination
- Principle of Least Authority (POLA)
- Practical applications: embedded systems (KeyKOS, seL4), distributed services, smart contracts

Key concepts relevant to Aspen:

- **Revokers**: Intermediary objects that can disable delegated access — maps to Aspen's capability token revocation
- **Attenuation**: Reducing scope of authority (read-only from read-write) — maps to token delegation with reduced permissions
- **Ambient authority is the enemy**: Programs should start with zero access and receive only what they need
- **Compositionality**: Building new capabilities by combining existing ones
