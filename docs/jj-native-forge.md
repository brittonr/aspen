# JJ-native Forge workflow

Aspen Forge can advertise Git-compatible and JJ-native repository backends independently. Git remains the default for legacy repositories. JJ support is opt-in through a Forge plugin that declares the JJ-native protocol route.

## Enable JJ routing on a node

1. Build or obtain a Forge plugin manifest that declares the protocol identifier `/aspen/forge/jj/1`.
2. Install the plugin with `aspen-cli plugin install --manifest <manifest.json> <plugin.wasm>`.
3. Keep the manifest `enabled` field true. Disabled manifests are ignored by Forge capability discovery.
4. Verify repository discovery with `aspen-cli forge repo get <repo-id>` or list output. Active JJ nodes report a `Jj` backend route with the same transport identifier.

Protocol identifiers are unique across enabled plugins. Activation or install fails when two enabled plugins claim the same identifier, so clients never receive ambiguous JJ routes.

## Create repositories

- Repositories without an explicit backend manifest behave as Git-only repositories.
- Dual-backend repositories should declare both `Git` and `Jj` in repository metadata before advertising JJ routes.
- JJ-only repositories should omit Git routes and reject Git-compatible access explicitly.

JJ namespaces are separate from Git refs:

- Git refs stay under the existing Git ref namespace.
- JJ bookmarks use `forge:jj:bookmark:<repo-id>:<bookmark>`.
- JJ change-id heads use `forge:jj:change:<repo-id>:<change-id>`.
- JJ object reachability uses `forge:jj:reach:<repo-id>:<object-id>`.

This separation prevents Git refs and JJ bookmarks from colliding in dual-backend repositories.

## Connect JJ clients

1. Resolve repository metadata with Forge discovery.
2. Select a route whose backend is `Jj` and whose transport identifier is `/aspen/forge/jj/1`.
3. Send a JJ-native admission request with transport version `1` before exchanging objects.
4. Treat `IncompatibleTransportVersion` as a hard failure and display the advertised server range.
5. Treat `CapabilityUnavailable` as a hard failure. Do not fall back to Git when the user requested JJ-native access.

## Push behavior

JJ pushes are staged first. Forge validates object envelopes, object graph consistency, current bookmark/change heads, quota, and repository availability before publishing repo-visible heads. Failed, timed-out, rejected, or abandoned staged sessions are cleaned without moving visible JJ heads.

## Disable or remove JJ support

Disable the plugin or remove the repository JJ backend before stopping JJ service on a node. Discovery then stops advertising JJ routes for that node. In-flight JJ publishes are rejected once repository deletion starts or JJ support is disabled.
