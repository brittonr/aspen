
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExecutionMetadata {
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use chrono::{NaiveDate, NaiveTime};

    prop_compose! {
        fn arb_datetime_utc()(
            year in 1970i32..2100i32,
            month in 1u32..12u32,
            day in 1u32..28u32, // To avoid issues with different month lengths
            hour in 0u32..23u32,
            minute in 0u32..59u32,
            second in 0u32..59u32,
            nanosecond in 0u32..999_999_999u32,
        ) -> DateTime<Utc> {
            let naive_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let naive_time = NaiveTime::from_hms_nano_opt(hour, minute, second, nanosecond).unwrap();
            let naive_datetime = naive_date.and_time(naive_time);
            DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc)
        }
    }

    proptest! {
        #[test]
        fn test_execution_metadata_creation(
            created_at in arb_datetime_utc(),
            started_at in proptest::option::of(arb_datetime_utc()),
            finished_at in proptest::option::of(arb_datetime_utc()),
        ) {
            let metadata = ExecutionMetadata {
                created_at: created_at.clone(),
                started_at: started_at.clone(),
                finished_at: finished_at.clone(),
            };
            assert_eq!(metadata.created_at, created_at);
            assert_eq!(metadata.started_at, started_at);
            assert_eq!(metadata.finished_at, finished_at);
        }
    }
}
