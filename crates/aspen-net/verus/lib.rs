//! Verus formal specifications for aspen-net.
//!
//! # Invariants Verified
//!
//! ## Service Name Validation
//!
//! 1. **SVCNAME-1: Length Boundedness**: valid service names contain 1 to 253 bytes.
//! 2. **SVCNAME-2: Leading Byte Admission**: valid names start with lowercase ASCII or digit.
//! 3. **SVCNAME-3: Body Byte Admission**: valid names contain only lowercase ASCII, digits, dots, or hyphens.
//! 4. **SVCNAME-4: Rejection Completeness**: empty, overlong, or malformed byte strings are rejected.

mod service_name_spec;
