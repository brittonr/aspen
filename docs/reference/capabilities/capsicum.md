# Capsicum: Practical Capabilities for UNIX

- **Authors**: Robert N. M. Watson, Jonathan Anderson (University of Cambridge Computer Laboratory)
- **URL**: https://www.cl.cam.ac.uk/research/security/capsicum/

Lightweight OS capability and sandbox framework that extends POSIX with object-capability security on UNIX-like operating systems. Developed at Cambridge with support from Google, the FreeBSD Foundation, and DARPA. Ships in FreeBSD 9.0+.

## Core Primitives

- **Capabilities**: Refined file descriptors with fine-grained rights
- **Capability mode**: Process sandbox that denies access to global namespaces
- **Process descriptors**: Capability-centric replacement for process IDs
- **Anonymous shared memory objects**: POSIX shared memory extended with fd-associated swap objects

## Supporting Infrastructure

- **rtld-elf-cap**: Modified ELF run-time linker for constructing sandboxed applications
- **libcapsicum**: Library for creating and using capabilities and sandboxed components
- **libuserangel**: Library for sandboxed apps to interact with user angels (e.g., Power Boxes for file open dialogs)
- **chromium-capsicum**: Chromium web browser using capability mode for renderer sandboxing

## Real-World Adoption

Self-compartmentalization applied to tcpdump, gzip, OpenSSH, dhclient, and Chromium — replacing porous `chroot()` sandboxes with strong capability-mode sandboxes.

## Key Concepts Relevant to Aspen

- **File descriptors as capabilities**: Capsicum treats fds as unforgeable tokens with fine-grained rights — same pattern as Aspen's capability tokens carrying specific permissions
- **Capability mode (ambient authority removal)**: Processes enter a mode where global namespaces (filesystem, PIDs, network) are inaccessible — parallels Aspen's principle that services start with zero access and receive only granted capabilities
- **Self-compartmentalization**: Existing programs retrofitted to sandbox their own risky components — relevant to Aspen's plugin isolation (Hyperlight) and job execution sandboxing
- **Rights attenuation on delegation**: Capabilities can be passed with reduced rights, never escalated — matches Aspen's token delegation model where delegated tokens can only narrow scope
