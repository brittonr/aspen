//! Opt-in operator smoke against explicitly supplied, independently published artifacts.
use std::env;
use std::path;

use super::super::status;

#[test]
#[ignore = "requires real independently pinned Mantle/Octet/ChaosControl artifacts"]
fn configured_replay_smoke() {
    let root = cap_std::fs::Dir::open_ambient_dir(
        env::var_os("SPACEWASM_ROOT").expect("SPACEWASM_ROOT"),
        cap_std::ambient_authority(),
    )
    .expect("explicit artifact root");
    let output = cap_std::fs::Dir::open_ambient_dir(
        env::var_os("SPACEWASM_OUTPUT").expect("SPACEWASM_OUTPUT"),
        cap_std::ambient_authority(),
    )
    .expect("explicit empty output directory");
    let left = path::PathBuf::from(env::var_os("SPACEWASM_LEFT").expect("SPACEWASM_LEFT"));
    let right = path::PathBuf::from(env::var_os("SPACEWASM_RIGHT").expect("SPACEWASM_RIGHT"));
    let expected_terminal = env::var("SPACEWASM_EXPECTED_TERMINAL").expect("SPACEWASM_EXPECTED_TERMINAL");
    let profiles = [
        super::configuration(&root, &left).unwrap().profile.content_ref().unwrap(),
        super::configuration(&root, &right).unwrap().profile.content_ref().unwrap(),
    ];
    let value = super::replay_configuration(&root, &left, &right, &output).expect("actual NASA replay");
    let identity = crate::codec::canonical_hash(&value).unwrap();
    let replay = crate::codec::simple_record_fields(&value, "molten-spacewasm-fixture-replay-status-v1", 5).unwrap();
    for side in [0, 1] {
        let report = crate::codec::value_to_iovalue(&replay[side]);
        let report = crate::codec::simple_record_fields(&report, "molten-spacewasm-fixture-status-v1", 25).unwrap();
        assert_eq!(report[13].as_boolean(), Some(true), "actual fixture acceptance");
        assert_eq!(report[19].as_boolean(), Some(true), "actual post-terminal cleanup");
        let outcome = crate::codec::value_to_iovalue(&report[14]);
        if expected_terminal == "finished" {
            crate::codec::simple_record_fields(&outcome, "finished", 0).expect("actual successful terminal");
        } else {
            let failure = crate::codec::simple_record_fields(&outcome, "failed", 1).expect("actual failure terminal");
            let failure = crate::codec::value_to_iovalue(&failure[0]);
            let failure = crate::codec::simple_record_fields(&failure, "spacewasm-failure-v1", 2).unwrap();
            assert_eq!(failure[0].as_string().as_deref(), Some(expected_terminal.as_str()));
        }
    }
    let readback = status::readback(&output, path::Path::new("status.preserves"), status::Expected {
        kind: status::Kind::Replay,
        receipt: &identity,
        profiles: &[&profiles[0], &profiles[1]],
    })
    .expect("canonical replay receipt and independently selected profiles");
    assert_eq!(readback.value(), &value);
    println!("{}", readback.render());
}
