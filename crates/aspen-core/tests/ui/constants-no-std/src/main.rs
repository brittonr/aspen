use aspen_constants::api::MAX_KEY_SIZE;
use aspen_constants::api::MAX_VALUE_SIZE;
use aspen_constants::network::IROH_CONNECT_TIMEOUT_SECS;

fn main() {
    assert!(MAX_KEY_SIZE > 0);
    assert!(MAX_VALUE_SIZE > 0);
    assert!(IROH_CONNECT_TIMEOUT_SECS > 0);
}
