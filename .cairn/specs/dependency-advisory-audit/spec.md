# Dependency Advisory Audit Specification

## Purpose

Defines the `dependency-advisory-audit` capability.

## Requirements

### Requirement: Release advisory observations bind exact selected closure

r[molten.dependency_audit.resolved] The default release audit MUST run a checksum-pinned maintained scanner against an explicit immutable RustSec database and MUST retain bounded raw machine output, package/version/source findings, paths, SPDX expressions and notice bytes, exact source/build/lock/target/default-feature bindings and a compact summary. Missing metadata, unknown source, stale scanner/data/binding and unavailable notices MUST deny or be explicitly unavailable. Existing release source and owned-license guards MUST remain prerequisites; audit evidence MUST NOT grant release, legal or runtime authority.

#### Scenario: Real scanner and immutable observation

- GIVEN an explicit maintained RustSec revision and pinned scanner
- WHEN the default target profile is captured offline
- THEN the report binds actual raw scan and input bytes and does not accept a supplied empty advisory list as a scan.

### Requirement: Negative controls and narrowly reviewed exceptions

r[molten.dependency_audit.controls] The audit MUST detect affected packages, source-policy violations, unavailable licenses, expired exceptions and stale bindings. An advisory exception MUST name exact package/version/source/advisory, owner, rationale, review identity and expiry; expired or invalid review MUST deny.

#### Scenario: Missing evidence never becomes clean

- GIVEN missing license metadata or stale required input
- WHEN the audit is evaluated
- THEN the machine report explicitly denies and identifies the unavailable or stale input.
