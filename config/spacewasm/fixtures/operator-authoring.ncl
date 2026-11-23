let C = import "../contracts.ncl" in
let checked = fun value => value | C.OperatorConfiguration in
# Deliberately non-admitted identities and unavailable executables, for authoring tests only.
let digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let verifier = fun name => {
  path = "/nix/store/00000000000000000000000000000000-spacewasm-authoring-fixture/bin/%{name}",
  blake3 = digest,
} in
let pins = {
  mantle = verifier "mantle",
  octet = verifier "octet",
  chaos = verifier "chaos",
  bundle_identity = digest,
  octet_profile_identity = "b3:%{digest}",
  cohort_identity = digest,
  probe_blake3 = digest,
} in
let selected = {
  profile = import "../profile.ncl",
  trust = pins,
  bundle = "bundle",
  octet = "evidence/octet",
  probe = "bin/spacewasm-probe",
  input = "fixtures/input.preserves",
  chaos_report = "out/chaos.json",
} in
let make_case = fun member step => selected & {
  name = member,
  scenario = step,
  module = "bundle/fixtures/wasm/%{member}.wasm",
} in
let plain_case = make_case "molten-abi-roundtrip" "Standard" in
let host = make_case "molten-abi-hostcall" in
{
  accepted = {
    # The name and located member must independently agree at execution.
    standard = checked plain_case,
    stale = checked (make_case "molten-abi-roundtrip" "StaleGeneration"),
    pause = checked (host "HostPause"),
    trap = checked (host { HostTrap = { reason = "fixture-host-trap" } }),
    uncertain = checked (host { HostUncertain = { reason = "fixture-after-commit" } }),
    cancel = checked (host { Cancel = { reason = "fixture-cancel", after_segments = 0 } }),
  },
  denied = {
    missing_cohort = checked (plain_case & { trust | force = std.record.remove "cohort_identity" pins }),
    missing_probe = checked (plain_case & { trust | force = std.record.remove "probe_blake3" pins }),
    malformed_verifier_digest = checked (plain_case & { trust.mantle.blake3 | force = "not-a-digest" }),
    ambient_verifier = checked (plain_case & { trust.chaos.path | force = "/usr/bin/chaos" }),
    wrong_octet_identity = checked (plain_case & { trust.octet_profile_identity | force = digest }),
    traversal = checked (plain_case & { module | force = "fixtures/../module.wasm" }),
    unexpected_field = checked (plain_case & { candidate_supplies_trust = true }),
    wrong_engine = checked (plain_case & { profile.name | force = "wasmtime-core" }),
    promote = checked (plain_case & { profile.production | force = true }),
    wrong_host_fixture = checked (make_case "molten-abi-roundtrip" "HostPause"),
  },
}
