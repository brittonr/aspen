{
  pkgs,
  profileDir,
  workspaceSource,
  savedEvidenceDir ? null,
}:

let
  profile = builtins.fromJSON (builtins.readFile (profileDir + "/generated/profile.json"));
  verifierVersion = builtins.replaceStrings [ "verus@" ] [ "" ] profile.current_probe.verifier;
  rustVersion = profile.current_probe.rust_version;
  rustTarget = "x86_64-unknown-linux-gnu";
  rustToolchainLabel = "${rustVersion}-${rustTarget}";
  rustupRunPrefixArgCount = 2;
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
  rustCompilerRelease = pkgs.fetchzip {
    url = "https://static.rust-lang.org/dist/rustc-${rustVersion}-${rustTarget}.tar.xz";
    hash = profile.current_probe.rust_compiler_source_hash;
  };
  rustStdRelease = pkgs.fetchzip {
    url = "https://static.rust-lang.org/dist/rust-std-${rustVersion}-${rustTarget}.tar.xz";
    hash = profile.current_probe.rust_std_source_hash;
  };
  rustToolchain = pkgs.runCommand "molten-node-replication-rust-${rustVersion}" { } ''
    mkdir -p "$out"
    cp -R ${rustCompilerRelease}/rustc/. "$out/"
    chmod -R u+w "$out"
    cp -R ${rustStdRelease}/rust-std-${rustTarget}/. "$out/"
  '';
  verifier = pkgs.stdenvNoCC.mkDerivation {
    pname = "molten-node-replication-verus";
    version = verifierVersion;
    src = pkgs.fetchurl {
      url = "https://github.com/verus-lang/verus/releases/download/release/${verifierVersion}/verus-${verifierVersion}-x86-linux.zip";
      hash = profile.current_probe.verifier_release_nix_sri;
    };
    nativeBuildInputs = with pkgs; [ autoPatchelfHook b3sum findutils makeWrapper unzip ];
    buildInputs = [ pkgs.stdenv.cc.cc.lib pkgs.zlib rustToolchain ];
    dontUnpack = true;
    preFixup = ''
      addAutoPatchelfSearchPath ${rustToolchain}/lib
    '';
    installPhase = ''
      runHook preInstall
      unpack_dir="$TMPDIR/verus-release"
      release_dir="$out/libexec/verus-release"
      mkdir -p "$unpack_dir" "$release_dir" "$out/bin" "$out/share"
      unzip -q "$src" -d "$unpack_dir"
      release_binary="$(find "$unpack_dir" -type f -name verus -print -quit)"
      test -n "$release_binary"
      release_root="$(dirname "$release_binary")"
      cp -R "$release_root/." "$release_dir/"
      chmod -R u+w "$release_dir"

      rustup_shim="$release_dir/rustup"
      cat > "$rustup_shim" <<'SHIM'
      #!${pkgs.runtimeShell}
      set -eu
      case "''${1:-}" in
        --version|-V)
          printf 'rustup 1.28.2 (Molten node-replication pilot shim)\n'
          ;;
        toolchain)
          if [ "''${2:-}" = "list" ]; then
            printf '${rustToolchainLabel} (default)\n'
          else
            echo "Molten pilot rustup shim: unsupported toolchain ''${2:-}" >&2
            exit 1
          fi
          ;;
        run)
          requested="''${2:-}"
          shift ${toString rustupRunPrefixArgCount}
          if [ "''${1:-}" = "--" ]; then
            shift
          fi
          case "$requested" in
            ${rustToolchainLabel}|${rustVersion}) ;;
            *) echo "Molten pilot rustup shim: unsupported toolchain $requested" >&2; exit 1 ;;
          esac
          export LD_LIBRARY_PATH="${rustToolchain}/lib:''${LD_LIBRARY_PATH:-}"
          exec "$@"
          ;;
        which)
          case "''${2:-}" in
            rustc) printf '%s/bin/rustc\n' "${rustToolchain}" ;;
            *) echo "Molten pilot rustup shim: unsupported which ''${2:-}" >&2; exit 1 ;;
          esac
          ;;
        show)
          if [ "''${2:-}" = "active-toolchain" ]; then
            printf '${rustToolchainLabel} (default)\n'
          else
            printf '${rustToolchainLabel}\n'
          fi
          ;;
        *) echo "Molten pilot rustup shim: unsupported: $*" >&2; exit 1 ;;
      esac
      SHIM
      chmod +x "$rustup_shim"
      makeWrapper "$release_dir/verus" "$out/libexec/verus-run" \
        --prefix PATH : "$release_dir" \
        --prefix LD_LIBRARY_PATH : "${rustToolchain}/lib" \
        --prefix LD_LIBRARY_PATH : "${pkgs.zlib}/lib" \
        --set VERUS_Z3_PATH "$release_dir/z3"

      verifier_blake3="$(b3sum "$release_dir/verus")"
      verifier_blake3="''${verifier_blake3%% *}"
      solver_blake3="$(b3sum "$release_dir/z3")"
      solver_blake3="''${solver_blake3%% *}"
      toolchain_blake3="$(b3sum ${rustToolchain}/bin/rustc)"
      toolchain_blake3="''${toolchain_blake3%% *}"
      test "$verifier_blake3" = '${profile.current_probe.verifier_binary_blake3}'
      test "$solver_blake3" = '${profile.current_probe.solver_blake3}'
      cat > "$out/share/identity.txt" <<IDENTITY
      proof-verifier-profile: ${profile.current_probe.profile_id}
      proof-verifier: ${profile.current_probe.verifier}
      proof-verifier-release-sha256: ${profile.current_probe.verifier_release_sha256_hex}
      proof-verifier-binary-blake3: $verifier_blake3
      proof-toolchain: ${rustToolchainLabel}
      proof-toolchain-blake3: $toolchain_blake3
      proof-solver: z3@verus-${verifierVersion}
      proof-solver-blake3: $solver_blake3
      trust-boundary: compatibility probe only; no proof or runtime-admission claim
      IDENTITY
      cat > "$out/bin/molten-node-replication-verus" <<WRAPPER
      #!${pkgs.runtimeShell}
      set -eu
      if [ "\$#" -eq 1 ] && [ "\$1" = "--identity" ]; then
        cat "$out/share/identity.txt"
        exit 0
      fi
      exec "$out/libexec/verus-run" "\$@"
      WRAPPER
      chmod +x "$out/bin/molten-node-replication-verus"
      runHook postInstall
    '';
  };
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
    ] ++ [ verifier ];
  } ''
    set -eu
    profile_dir=${profileDir}
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

    molten-node-replication-verus --identity > "$output_dir/verifier-identity.txt"
    grep -F 'proof-verifier: ${profile.current_probe.verifier}' "$output_dir/verifier-identity.txt" > /dev/null
    entrypoint="$crate_root/src/lib.rs"
    set +e
    timeout '${toString profile.current_probe.timeout_seconds}' \
      molten-node-replication-verus --crate-type=lib "$entrypoint" \
      > "$TMPDIR/no-feature.stdout" 2> "$TMPDIR/no-feature.stderr"
    no_feature_status="$?"
    timeout '${toString profile.current_probe.timeout_seconds}' \
      molten-node-replication-verus -V new-mut-ref --crate-type=lib "$entrypoint" \
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
    toolchain_blake3="$(awk -F ': ' '$1 == "proof-toolchain-blake3" { print $2 }' "$output_dir/verifier-identity.txt")"

    jq --null-input \
      --arg trusted_blake3 "$trusted_blake3" \
      --arg external_blake3 "$external_blake3" \
      --arg assume_blake3 "$assume_blake3" \
      --arg current_diagnostic_blake3 "$current_diagnostic_blake3" \
      --arg no_feature_diagnostic_blake3 "$no_feature_diagnostic_blake3" \
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
          profile_id: "${profile.current_probe.profile_id}",
          verifier: "${profile.current_probe.verifier}",
          verifier_binary_blake3: "${profile.current_probe.verifier_binary_blake3}",
          solver_blake3: "${profile.current_probe.solver_blake3}",
          toolchain: "${rustToolchainLabel}",
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
