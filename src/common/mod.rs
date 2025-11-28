pub mod timestamp;

pub use timestamp::{
    SafeTimestamp, TimeError, TimeResult,
    get_unix_timestamp, get_unix_timestamp_or_zero,
    current_timestamp, current_timestamp_or_zero
};