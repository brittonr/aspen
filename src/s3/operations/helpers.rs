//! Shared helper functions for S3 operations.
//!
//! Contains utility functions used across multiple S3 operation modules,
//! including timestamp conversion, content type parsing, and range resolution.

use crate::s3::error::{S3Error, S3Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream;
use s3s::dto::{ContentType, Range, StreamingBlob, Timestamp};
use std::time::SystemTime;

/// Resolved byte range for partial content requests.
pub(crate) struct ResolvedRange {
    /// Start byte position (inclusive).
    pub start: u64,
    /// End byte position (inclusive).
    pub end: u64,
}

/// Convert chrono DateTime to s3s Timestamp.
pub(crate) fn chrono_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    // Convert to SystemTime
    let duration = dt.timestamp() as u64;
    let nanos = dt.timestamp_subsec_nanos();
    let system_time = SystemTime::UNIX_EPOCH + std::time::Duration::new(duration, nanos);
    Timestamp::from(system_time)
}

/// Parse a content type string into a ContentType.
pub(crate) fn parse_content_type(s: &str) -> Option<ContentType> {
    // ContentType wraps mime::Mime, parse the string as a Mime type
    s.parse::<mime::Mime>().ok()
}

/// Create a streaming blob from bytes.
pub(crate) fn bytes_to_streaming_blob(data: Vec<u8>) -> StreamingBlob {
    let bytes = Bytes::from(data);
    // Create a stream that yields one chunk
    let data_stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
    StreamingBlob::wrap(data_stream)
}

/// Resolve an HTTP Range header to actual byte positions.
///
/// Returns the resolved range (start, end) inclusive, or an error if
/// the range is not satisfiable.
pub(crate) fn resolve_range(range: &Range, object_size: u64) -> S3Result<ResolvedRange> {
    if object_size == 0 {
        return Err(S3Error::InvalidRange {
            reason: "Cannot request range of empty object".to_string(),
        });
    }

    match range {
        Range::Int { first, last } => {
            // first-last: bytes first to last (inclusive)
            // first-: bytes first to end
            if *first >= object_size {
                return Err(S3Error::InvalidRange {
                    reason: format!("Start position {} >= object size {}", first, object_size),
                });
            }

            let end = match last {
                Some(l) => (*l).min(object_size - 1),
                None => object_size - 1,
            };

            if end < *first {
                return Err(S3Error::InvalidRange {
                    reason: format!("End position {} < start position {}", end, first),
                });
            }

            Ok(ResolvedRange { start: *first, end })
        }
        Range::Suffix { length } => {
            // -length: last length bytes
            if *length == 0 {
                return Err(S3Error::InvalidRange {
                    reason: "Suffix length cannot be zero".to_string(),
                });
            }

            let start = object_size.saturating_sub(*length);
            Ok(ResolvedRange {
                start,
                end: object_size - 1,
            })
        }
    }
}

/// Format a Content-Range header value.
pub(crate) fn format_content_range(start: u64, end: u64, total: u64) -> String {
    format!("bytes {}-{}/{}", start, end, total)
}
