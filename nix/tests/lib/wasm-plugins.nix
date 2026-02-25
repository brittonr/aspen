# Shared helpers for NixOS VM tests that use WASM plugins.
#
# Provides:
#   - pluginFiles: NixOS config fragment placing WASM + manifests in /etc/aspen-plugins/
#   - pluginCli: wrapper script `aspen-plugin-cli` for plugin management
#   - installPluginsScript: Python test helper to install + reload all plugins
#
# Usage in a test .nix file:
#   let
#     pluginHelpers = import ./lib/wasm-plugins.nix {
#       inherit pkgs;
#       aspenCliPlugins = aspenCliPluginsPackage;
#       plugins = [
#         { name = "coordination"; wasm = coordinationPluginWasm; }
#       ];
#     };
#   in pkgs.testers.nixosTest {
#     nodes.node1 = {
#       imports = [ pluginHelpers.nixosConfig ];
#       ...
#     };
#     testScript = ''
#       ${pluginHelpers.installPluginsScript}
#       ...
#     '';
#   }
{
  pkgs,
  aspenCliPlugins,
  plugins, # list of { name, wasm } where wasm is the derivation from buildWasmPlugin
}: let
  lib = pkgs.lib;

  # Wrapper script that runs aspen-cli-plugins under a different binary name
  # so it doesn't conflict with the test-specific aspen-cli in PATH.
  pluginCli = pkgs.writeShellScriptBin "aspen-plugin-cli" ''
    exec ${aspenCliPlugins}/bin/aspen-cli "$@"
  '';

  # NixOS config fragment: places WASM files + manifests in /etc/aspen-plugins/
  nixosConfig = {
    config,
    lib,
    ...
  }: {
    environment.systemPackages = [pluginCli];

    environment.etc = lib.listToAttrs (
      lib.concatMap (p: [
        {
          name = "aspen-plugins/${p.name}-plugin.wasm";
          value.source = "${p.wasm}/${p.name}-plugin.wasm";
        }
        {
          name = "aspen-plugins/${p.name}-plugin.json";
          value.source = "${p.wasm}/plugin.json";
        }
      ])
      plugins
    );
  };

  # Python test script fragment that installs all WASM plugins and reloads.
  # Assumes:
  #   - get_ticket() helper exists in the test
  #   - The node variable is named `node1`
  #   - The cluster is already initialized
  pluginNames = map (p: p.name) plugins;
  installPluginsScript = ''
    # ── WASM plugin installation ─────────────────────────────────────
    def plugin_cli(cmd, check=True):
        """Run aspen-plugin-cli with cluster ticket, return parsed JSON."""
        import inspect
        _gt_sig = inspect.signature(get_ticket)
        ticket = get_ticket(node1) if _gt_sig.parameters else get_ticket()
        run = (
            f"aspen-plugin-cli --ticket '{ticket}' --json {cmd} "
            f">/tmp/_plugin_cli_out.json 2>/tmp/_plugin_cli_err.txt"
        )
        if check:
            node1.succeed(run)
        else:
            node1.execute(run)
        raw = node1.succeed("cat /tmp/_plugin_cli_out.json")
        try:
            return json.loads(raw)
        except (json.JSONDecodeError, ValueError):
            node1.log(f"plugin_cli() parse failed: {raw!r}")
            return raw.strip()

    with subtest("install WASM plugins"):
        _plugin_specs = [
            ${lib.concatMapStringsSep "\n            " (name: ''("${name}", "/etc/aspen-plugins/${name}-plugin.wasm", "/etc/aspen-plugins/${name}-plugin.json"),'') pluginNames}
        ]
        for _pname, _pwasm, _pmanifest in _plugin_specs:
            out = plugin_cli(
                f"plugin install {_pwasm} --manifest {_pmanifest}"
            )
            node1.log(f"installed {_pname} plugin: {out}")
            assert isinstance(out, dict) and out.get("status") == "installed", \
                f"{_pname} plugin install failed: {out}"

    with subtest("reload plugin runtime"):
        out = plugin_cli("plugin reload", check=False)
        node1.log(f"plugin reload: {out}")
        if isinstance(out, dict):
            assert out.get("is_success", False), \
                f"plugin reload failed: {out.get('error', out.get('message', out))}"
            node1.log(f"plugin reload succeeded: {out.get('plugin_count', 0)} plugin(s)")
        else:
            node1.log(f"plugin reload returned non-JSON (may have failed): {out}")
        time.sleep(5)

        # Verify plugins loaded via list (reads KV manifests)
        out = plugin_cli("plugin list")
        node1.log(f"loaded plugins: {out}")
        expected_count = ${toString (builtins.length plugins)}
        actual_count = out.get("count", 0)
        assert actual_count >= expected_count, \
            f"expected >= {expected_count} plugins, got {actual_count}: {out}"
        node1.log(f"All {actual_count} WASM plugins loaded successfully")
  '';
in {
  inherit pluginCli nixosConfig installPluginsScript;
}
