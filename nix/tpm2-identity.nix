# Repository-pinned nixpkgs supplies the tpm2-tools and swtpm versions recorded below.
# This is a disposable check module, not a NixOS daemon backend selector.
{ pkgs, testBinaries, rustLibDir ? null }:
let
  # Retained fd execution changes argv[0]. Enter the immutable Nix wrapper by its
  # real basename so tpm2's multicall dispatch still sees "tpm2". No secret here.
  tool = pkgs.writeShellScript "molten-tpm2-tool" ''
    exec ${pkgs.tpm2-tools}/bin/tpm2 "$@"
  '';
in
pkgs.runCommand "molten-tpm2-sealed-identity"
  {
    nativeBuildInputs = [ pkgs.swtpm pkgs.coreutils pkgs.gnugrep ];
    inherit testBinaries;
  }
  ''
    set -euo pipefail
    umask 077
    export MOLTEN_TPM2_FIXTURE_ROOT="$TMPDIR/tpm2-fixture"
    export MOLTEN_TPM2_TOOL=${tool}
    # Provider device state is not consumer filesystem authority. A software TPM
    # can serialize its sensitive objects in this private, unencrypted directory.
    # Keep it a sibling, never a child excluded by the consumer's plaintext scan.
    provider_state="$TMPDIR/tpm2-provider-state"
    mkdir -p "$MOLTEN_TPM2_FIXTURE_ROOT" "$provider_state"
    ${pkgs.swtpm}/bin/swtpm socket --tpm2 \
      --tpmstate "dir=$provider_state" \
      --server "type=unixio,path=$MOLTEN_TPM2_FIXTURE_ROOT/swtpm.sock" \
      --ctrl "type=unixio,path=$MOLTEN_TPM2_FIXTURE_ROOT/swtpm.sock.ctrl" \
      --flags not-need-init,startup-clear &
    emulator=$!
    trap 'kill "$emulator" 2>/dev/null || true; wait "$emulator" 2>/dev/null || true' EXIT
    ready=0
    for attempt in $(seq 1 100); do
      kill -0 "$emulator"
      if [ -S "$MOLTEN_TPM2_FIXTURE_ROOT/swtpm.sock" ]; then ready=1; break; fi
      sleep 0.05
    done
    test "$ready" -eq 1
    ${pkgs.lib.optionalString (rustLibDir != null) ''export LD_LIBRARY_PATH=${pkgs.lib.escapeShellArg rustLibDir}''}
    test_name=fabric_crypto_identity::tpm2::tests::sealed::load_pcr_denial_secrecy_and_rotation
    selected=
    count=0
    for binary in "$testBinaries"/tests/*; do
      test -f "$binary" && test -x "$binary" || continue
      if "$binary" --list --ignored --exact "$test_name" > "$TMPDIR/test-list" 2>/dev/null \
        && grep -Fxq "$test_name: test" "$TMPDIR/test-list"; then
        selected="$binary"
        count=$((count + 1))
      fi
    done
    test "$count" -eq 1
    status=0
    timeout 180 "$selected" --ignored --exact "$test_name" --nocapture > "$TMPDIR/consumer.log" 2>&1 || status=$?
    cat "$TMPDIR/consumer.log"
    test "$status" -eq 0 || exit "$status"
    mkdir -p "$out"
    cp "$TMPDIR/consumer.log" "$out/consumer.log"
    printf '%s\n' 'tpm2-tools=${pkgs.tpm2-tools.version}' 'swtpm=${pkgs.swtpm.version}' \
      'PCR=sha256:16 (actually extended before denial)' \
      'filesystem=all TPM consumer regular files scanned; provider device state is a separate authority' \
      'provider-state=unencrypted disposable software TPM; no host-filesystem secrecy claim' \
      'scope=disposable swtpm; no hardware/attestation/production qualification' > "$out/cohort.txt"
  ''
