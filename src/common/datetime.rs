use prisma_client_rust::chrono::{DateTime, FixedOffset, Utc};

pub fn ts_to_datetime_fixed(ts: i64) -> DateTime<FixedOffset> {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&FixedOffset::east_opt(0).unwrap()))
        .unwrap()
}
