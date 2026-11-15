#[path = "src/test/support.rs"]
mod test_support;

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/worldsnapshot/fixture.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/worldsnapshot/cases.rs"));
