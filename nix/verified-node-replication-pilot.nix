{
  pkgs,
  octetPackages,
  octetRevision,
  profileDir,
  workspaceSource,
  savedEvidenceDir ? null,
}:

let
  profile = builtins.fromJSON (builtins.readFile (profileDir + "/generated/profile.json"));
  rustToolchainLabel = "${profile.current_probe.rust_version}-x86_64-unknown-linux-gnu";
  expectedProbeFailureExitCode = 1;
  timeoutExitCode = 124;
  source = pkgs.fetchzip {
    url = "${profile.upstream.repository_url}/archive/${profile.upstream.revision}.tar.gz";
    hash = profile.upstream.source_hash;
  };
  sourceTreeEvidence = pkgs.fetchurl {
    url = "https://api.github.com/repos/verus-lang/verified-node-replication/git/trees/${profile.upstream.revision}?recursive=1";
    hash = profile.upstream.tree_api_hash;
  };
  historicalVerusSource = pkgs.fetchzip {
    url = "https://github.com/verus-lang/verus/archive/${profile.historical_verus.revision}.tar.gz";
    hash = profile.historical_verus.source_hash;
  };
  verusToolchainProfile = builtins.getAttr "octet-verus-toolchain-profile" octetPackages;
  verifier = builtins.getAttr "octet-production-verus" octetPackages;
  check = pkgs.runCommand "molten-verified-node-replication-pilot" {
    nativeBuildInputs = with pkgs; [
      b3sum
      coreutils
      diffutils
      gawk
      gnugrep
      jq
      nickel
      ripgrep
    ] ++ [ verusToolchainProfile verifier ];
  } ''
    set -eu
    profile_dir=${profileDir}
    octet_profile_json=${verusToolchainProfile}/share/octet/verus-toolchain/profile.json
    upstream=${source}
    crate_root="$upstream/verified-node-replication"
    output_dir="$out/share/molten/verified-node-replication-pilot"
    mkdir -p "$output_dir"

    cd "$profile_dir"
    nickel export --format json profile.ncl > "$TMPDIR/profile.json"
    diff --unified generated/profile.json "$TMPDIR/profile.json"
    nickel export --format json fixtures/valid/current_profile.ncl > /dev/null
    for invalid_fixture in fixtures/invalid/*.ncl; do
      if nickel export --format json "$invalid_fixture" > /dev/null 2>&1; then
        echo "invalid node-replication pilot fixture unexpectedly passed: $invalid_fixture" >&2
        exit 1
      fi
    done
    validate_octet_profile() {
      jq --exit-status '
        .schema_version == "octet-verus-toolchain-profile/v1"
        and .profile_id == "${profile.current_probe.profile_id}"
        and .verifier.release == "${builtins.replaceStrings [ "verus@" ] [ "" ] profile.current_probe.verifier}"
        and .verifier.release_nix_sri == "${profile.current_probe.verifier_release_nix_sri}"
        and .verifier.rust_toolchain_label == "${rustToolchainLabel}"
        and any(.compatibility[];
          .consumer == "aspen-node-replication-pilot"
          and .status == "native"
          and .verifier_release == "${builtins.replaceStrings [ "verus@" ] [ "" ] profile.current_probe.verifier}")
      ' "$1" > /dev/null
    }
    validate_octet_profile "$octet_profile_json"
    jq '(.compatibility[] | select(.consumer == "aspen-node-replication-pilot")).status = "unsupported"' \
      "$octet_profile_json" > "$TMPDIR/mismatched-octet-profile.json"
    if validate_octet_profile "$TMPDIR/mismatched-octet-profile.json"; then
      echo "mismatched Octet Aspen compatibility profile unexpectedly passed" >&2
      exit 1
    fi
    cp "$octet_profile_json" "$output_dir/octet-profile.json"

    ${pkgs.lib.concatMapStringsSep "\n" (sentinel: ''
      test -e "$upstream/${sentinel}"
    '') profile.upstream.sentinels}
    grep -F 'MIT License' "$upstream/LICENSE" > /dev/null
    jq --exit-status --arg revision '${profile.historical_verus.revision}' '
      .tree[] | select(.path == "verus" and .type == "commit" and .sha == $revision)
    ' ${sourceTreeEvidence} > /dev/null
    test -f ${historicalVerusSource}/rust-toolchain.toml
    grep -F 'channel = "${profile.historical_verus.rust_version}"' \
      ${historicalVerusSource}/rust-toolchain.toml > /dev/null
    test -d ${historicalVerusSource}/source/rust_verify

    if rg --fixed-strings 'verified-node-replication' \
      --glob '**/Cargo.toml' --glob 'Cargo.lock' ${workspaceSource} > "$TMPDIR/runtime-dependency-matches.txt"; then
      echo "verified-node-replication unexpectedly entered the Molten runtime dependency graph" >&2
      cat "$TMPDIR/runtime-dependency-matches.txt" >&2
      exit 1
    fi

    trusted_markers="$output_dir/trusted-markers.txt"
    external_bodies="$output_dir/external-bodies.txt"
    assume_sites="$output_dir/assume-sites.txt"
    rg --line-number --no-heading --fixed-strings '#[verus::trusted]' "$crate_root/src" \
      | sed "s#^$upstream/##" | LC_ALL=C sort > "$trusted_markers"
    {
      rg --line-number --no-heading --fixed-strings '#[verifier::external_body]' "$crate_root/src" || true
      rg --line-number --no-heading --fixed-strings '#[verifier(external_body)]' "$crate_root/src" || true
    } | sed "s#^$upstream/##" | LC_ALL=C sort > "$external_bodies"
    rg --line-number --no-heading --fixed-strings 'assume(' "$crate_root/src" \
      | sed "s#^$upstream/##" | LC_ALL=C sort > "$assume_sites"
    trusted_count="$(wc -l < "$trusted_markers")"
    external_body_count="$(wc -l < "$external_bodies")"
    assume_count="$(wc -l < "$assume_sites")"
    test "$trusted_count" -eq '${toString profile.trusted_boundary.trusted_marker_count}'
    test "$external_body_count" -eq '${toString profile.trusted_boundary.external_body_count}'
    test "$assume_count" -eq '${toString profile.trusted_boundary.assume_count}'
    ${pkgs.lib.concatMapStringsSep "\n" (location:
      let
        parts = pkgs.lib.splitString ":" location;
        sourcePath = builtins.elemAt parts 0;
        line = builtins.elemAt parts 1;
        symbol = builtins.elemAt parts 2;
      in ''
        sed -n '${line}p' "$upstream/${sourcePath}" | grep -F '#[' > /dev/null
        rg --fixed-strings '${symbol}' "$upstream/${sourcePath}" > /dev/null
      '') profile.trusted_boundary.required_locations}

    octet-production-verus --identity > "$output_dir/verifier-identity.txt"
    grep -F 'proof-verifier: ${profile.current_probe.verifier}' "$output_dir/verifier-identity.txt" > /dev/null
    identity_value() {
      identity_key="$1"
      identity_result="$(awk -F ': ' -v key="$identity_key" '$1 == key { sub(/^[^:]*: /, ""); print; exit }' "$output_dir/verifier-identity.txt")"
      test -n "$identity_result"
      printf '%s' "$identity_result"
    }
    verifier_binary_blake3="$(identity_value proof-verifier-binary-blake3)"
    solver_blake3="$(identity_value proof-solver-blake3)"
    test "$verifier_binary_blake3" = '${profile.current_probe.verifier_binary_blake3}'
    test "$solver_blake3" = '${profile.current_probe.solver_blake3}'
    entrypoint="$crate_root/src/lib.rs"
    set +e
    timeout '${toString profile.current_probe.timeout_seconds}' \
      octet-production-verus --crate-type=lib "$entrypoint" \
      > "$TMPDIR/no-feature.stdout" 2> "$TMPDIR/no-feature.stderr"
    no_feature_status="$?"
    timeout '${toString profile.current_probe.timeout_seconds}' \
      octet-production-verus -V new-mut-ref --crate-type=lib "$entrypoint" \
      > "$TMPDIR/current.stdout" 2> "$TMPDIR/current.stderr"
    current_status="$?"
    set -e
    test "$no_feature_status" -eq '${toString expectedProbeFailureExitCode}'
    test "$current_status" -eq '${toString expectedProbeFailureExitCode}'
    test "$no_feature_status" -ne '${toString timeoutExitCode}'
    test "$current_status" -ne '${toString timeoutExitCode}'

    feature_diagnostic="The verifier does not yet support the following Rust feature: mut_ref spec funs without '-V new-mut-ref'"
    feature_count="$(rg --only-matching --fixed-strings "$feature_diagnostic" "$TMPDIR/no-feature.stderr" | wc -l)"
    test "$feature_count" -eq '${toString profile.current_probe.unsupported_feature_diagnostic_count}'
    ${pkgs.lib.concatMapStringsSep "\n" (anchor: ''
      grep -F '${anchor}' "$TMPDIR/current.stderr" > /dev/null
    '') profile.current_probe.required_diagnostic_anchors}

    cat > "$output_dir/no-feature-diagnostic.txt" <<DIAGNOSTIC
    status=unsupported-feature
    diagnostic_count=$feature_count
    required_flag=-V new-mut-ref
    saved_log_scope=normalized-summary-only
    DIAGNOSTIC
    cat > "$output_dir/current-verifier-diagnostic.txt" <<DIAGNOSTIC
    status=internal-error
    diagnostic=Verus Internal Error: var_local_id failed
    source=verified-node-replication/src/exec/rwlock.rs
    decision=${profile.current_probe.expected_decision}
    saved_log_scope=normalized-summary-only
    DIAGNOSTIC
    head --bytes '${toString profile.current_probe.maximum_saved_log_bytes}' \
      "$TMPDIR/current.stderr" \
      | sed "s#${source}#<PINNED_UPSTREAM_SOURCE>#g" \
      | sed -E "s/thread 'rustc' \([0-9]+\)/thread 'rustc' (<PID>)/" \
      > "$output_dir/current-verifier.stderr.normalized"
    test "$(wc -c < "$output_dir/current-verifier.stderr.normalized")" \
      -le '${toString profile.current_probe.maximum_saved_log_bytes}'

    trusted_blake3="$(b3sum "$trusted_markers")"
    trusted_blake3="''${trusted_blake3%% *}"
    external_blake3="$(b3sum "$external_bodies")"
    external_blake3="''${external_blake3%% *}"
    assume_blake3="$(b3sum "$assume_sites")"
    assume_blake3="''${assume_blake3%% *}"
    current_diagnostic_blake3="$(b3sum "$output_dir/current-verifier-diagnostic.txt")"
    current_diagnostic_blake3="''${current_diagnostic_blake3%% *}"
    no_feature_diagnostic_blake3="$(b3sum "$output_dir/no-feature-diagnostic.txt")"
    no_feature_diagnostic_blake3="''${no_feature_diagnostic_blake3%% *}"
    octet_profile_blake3="$(b3sum "$octet_profile_json")"
    octet_profile_blake3="''${octet_profile_blake3%% *}"
    toolchain="$(awk -F ': ' '$1 == "proof-toolchain" { print $2 }' "$output_dir/verifier-identity.txt")"
    toolchain_blake3="$(awk -F ': ' '$1 == "proof-toolchain-blake3" { print $2 }' "$output_dir/verifier-identity.txt")"
    test "$toolchain" = '${rustToolchainLabel}'

    jq --null-input \
      --arg trusted_blake3 "$trusted_blake3" \
      --arg external_blake3 "$external_blake3" \
      --arg assume_blake3 "$assume_blake3" \
      --arg current_diagnostic_blake3 "$current_diagnostic_blake3" \
      --arg no_feature_diagnostic_blake3 "$no_feature_diagnostic_blake3" \
      --arg octet_profile_blake3 "$octet_profile_blake3" \
      --arg octet_profile_nix_output '${verusToolchainProfile}' \
      --arg octet_revision '${octetRevision}' \
      --arg verifier_binary_blake3 "$verifier_binary_blake3" \
      --arg solver_blake3 "$solver_blake3" \
      --arg toolchain "$toolchain" \
      --arg toolchain_blake3 "$toolchain_blake3" \
      --arg verifier_nix_output '${verifier}' \
      --argjson trusted_count "$trusted_count" \
      --argjson external_body_count "$external_body_count" \
      --argjson assume_count "$assume_count" \
      '{
        payload_schema: "molten-verified-node-replication-pilot-decision-payload-v1",
        pilot_id: "${profile.pilot_id}",
        decision: "${profile.current_probe.expected_decision}",
        runtime_dependency_status: "denied",
        promotion_eligible: false,
        scope: "${profile.scope}",
        upstream: {
          project: "${profile.upstream.project}",
          revision: "${profile.upstream.revision}",
          source_hash: "${profile.upstream.source_hash}",
          source_hash_protocol: "${profile.upstream.source_hash_protocol}",
          license: "${profile.upstream.license}"
        },
        historical_verus: {
          revision: "${profile.historical_verus.revision}",
          source_hash: "${profile.historical_verus.source_hash}",
          rust_version: "${profile.historical_verus.rust_version}"
        },
        current_probe: {
          provider: "octet",
          provider_revision: $octet_revision,
          profile_id: "${profile.current_probe.profile_id}",
          profile_blake3: $octet_profile_blake3,
          profile_nix_output: $octet_profile_nix_output,
          verifier: "${profile.current_probe.verifier}",
          verifier_binary_blake3: $verifier_binary_blake3,
          solver_blake3: $solver_blake3,
          toolchain: $toolchain,
          toolchain_blake3: $toolchain_blake3,
          verifier_nix_output: $verifier_nix_output,
          without_required_flag: "unsupported-feature",
          with_required_flag: "internal-error",
          no_feature_diagnostic_blake3: $no_feature_diagnostic_blake3,
          current_diagnostic_blake3: $current_diagnostic_blake3
        },
        trusted_boundary: {
          trusted_marker_count: $trusted_count,
          trusted_markers_blake3: $trusted_blake3,
          external_body_count: $external_body_count,
          external_bodies_blake3: $external_blake3,
          assume_count: $assume_count,
          assume_sites_blake3: $assume_blake3,
          top_level_refinement_theorems_trusted: true,
          public_traits_trusted: true
        },
        unmet_promotion_criteria: ${builtins.toJSON profile.promotion_criteria},
        claim_scope: "${profile.claim_scope}",
        non_claims: ${builtins.toJSON profile.non_claims}
      }' > "$output_dir/decision-payload.json"
    jq --sort-keys --compact-output '.' "$output_dir/decision-payload.json" \
      > "$output_dir/decision-payload.canonical.json"
    decision_blake3="$(b3sum "$output_dir/decision-payload.canonical.json")"
    decision_blake3="''${decision_blake3%% *}"
    jq --null-input \
      --slurpfile payload "$output_dir/decision-payload.json" \
      --arg decision_blake3 "$decision_blake3" \
      '{
        schema_version: "molten-verified-node-replication-pilot-decision-v1",
        hash_scope: "blake3-of-jq-sorted-compact-payload-json",
        payload: $payload[0],
        decision_blake3: $decision_blake3
      }' | jq --sort-keys '.' > "$output_dir/decision.json"
    cp "$TMPDIR/profile.json" "$output_dir/profile.json"

    jq --exit-status '
      .payload.decision == "blocked-verifier-internal-error"
      and .payload.runtime_dependency_status == "denied"
      and .payload.promotion_eligible == false
      and .payload.scope == "local-multicore-numa-data-structure-only"
      and .payload.current_probe.with_required_flag == "internal-error"
      and .payload.trusted_boundary.top_level_refinement_theorems_trusted == true
      and (.payload.unmet_promotion_criteria | length > 0)
    ' "$output_dir/decision.json" > /dev/null
    jq --sort-keys --compact-output '.payload' "$output_dir/decision.json" \
      > "$TMPDIR/recomputed-payload.json"
    recomputed_blake3="$(b3sum "$TMPDIR/recomputed-payload.json")"
    recomputed_blake3="''${recomputed_blake3%% *}"
    jq --exit-status --arg digest "$recomputed_blake3" '.decision_blake3 == $digest' \
      "$output_dir/decision.json" > /dev/null
    ${pkgs.lib.optionalString (savedEvidenceDir != null) ''
      diff --unified ${savedEvidenceDir}/decision.json "$output_dir/decision.json"
      diff --unified ${savedEvidenceDir}/decision-payload.canonical.json "$output_dir/decision-payload.canonical.json"
      diff --unified ${savedEvidenceDir}/octet-profile.json "$output_dir/octet-profile.json"
      diff --unified ${savedEvidenceDir}/verifier-identity.txt "$output_dir/verifier-identity.txt"
      diff --unified ${savedEvidenceDir}/trusted-markers.txt "$output_dir/trusted-markers.txt"
      diff --unified ${savedEvidenceDir}/external-bodies.txt "$output_dir/external-bodies.txt"
      diff --unified ${savedEvidenceDir}/assume-sites.txt "$output_dir/assume-sites.txt"
      diff --unified ${savedEvidenceDir}/current-verifier-diagnostic.txt "$output_dir/current-verifier-diagnostic.txt"
      diff --unified ${savedEvidenceDir}/no-feature-diagnostic.txt "$output_dir/no-feature-diagnostic.txt"
      diff --unified ${savedEvidenceDir}/current-verifier.stderr.normalized "$output_dir/current-verifier.stderr.normalized"
    ''}
  '';
in
{
  inherit check verifier;
  inherit source historicalVerusSource;
}
