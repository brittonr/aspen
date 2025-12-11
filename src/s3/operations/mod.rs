//! Internal operation implementations for S3 API.
//!
//! Each module implements a subset of S3 operations as trait methods
//! on [`AspenS3Service`](crate::s3::AspenS3Service).
//!
//! Not part of the public API - users interact through `AspenS3Service`.
//!
//! # Module Organization
//!
//! - [`bucket`]: Bucket lifecycle operations (create, delete, list, head, location)
//! - [`object`]: Object operations (put, get, delete, head, copy, list)
//! - [`multipart`]: Multipart upload operations
//! - [`versioning`]: Bucket versioning configuration
//! - [`tagging`]: Object and bucket tagging
//! - [`lifecycle`]: Bucket lifecycle configuration
//! - [`encryption`]: Bucket encryption configuration
//! - [`helpers`]: Shared utility functions

pub(crate) mod bucket;
pub(crate) mod encryption;
pub(crate) mod helpers;
pub(crate) mod lifecycle;
pub(crate) mod multipart;
pub(crate) mod object;
pub(crate) mod tagging;
pub(crate) mod versioning;

// Note: Traits (BucketOps, ObjectOps, etc.) are used via impl blocks on
// AspenS3Service within each module, not through trait bounds.
