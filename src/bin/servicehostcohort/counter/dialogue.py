"""Bounded host dialogue and independent invocation signing for the counter flow."""
import json
import os
import re
import subprocess
import threading


def execute(command, root, config, args, temporary, authorizer, *,
            kill_after_native=False, expect_failure=False, revoke_current=False):
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               text=True, bufsize=1)
    deadline = threading.Timer(args.timeout, process.kill)
    deadline.start()
    result = None
    signing_requests = []
    try:
        while True:
            line = process.stdout.readline(65538)
            if not line:
                break
            if len(line) > 65537 or not line.endswith("\n"):
                raise RuntimeError("host dialogue exceeded its bounded single-line frame")
            if line.startswith("SIGN "):
                preview = line[len("SIGN "):]
                if kill_after_native and '"actor-native-commit"' in preview:
                    with (root / "negative-last-signing-request.preserves").open("x") as stream:
                        stream.write(preview)
                    process.kill()
                    process.wait(timeout=args.timeout)
                    result = {"semantic_outcome": "unknown", "injected_sigkill": True, "last_request": preview}
                    break
                preview_file = temporary / "request.preserves"
                preview_file.write_text(preview)
                nonce_file = temporary / "nonce"
                nonce_file.write_bytes(os.urandom(32))
                signed = subprocess.run([
                    str(authorizer), "authorize", "--cohort", str(root),
                    "--source-trust", str(args.source_trust), "--signed-output", str(args.signed_output),
                    "--issuer-key", str(args.issuer_key), "--holder-key", str(args.holder_key),
                    "--policy", str(args.policy), "--request", str(preview_file),
                    "--nonce-file", str(nonce_file), "--request-format", "text",
                    *(["--revoked-trust-output", "revoked-service-host-trust.preserves"] if revoke_current else []),
                ], check=True, capture_output=True, text=True, timeout=args.timeout)
                refs = re.findall(r'"(blake3:[0-9a-f]{64})"', signed.stdout)
                if len(refs) != 2 or refs[0] != config["bundle_ref"]:
                    raise RuntimeError("authorizer did not return the exact signed invocation receipt")
                if revoke_current:
                    # The independent external authorizer derived the actual token CID via its wire owner.
                    # Current rereads this exact public revocation list only after READY.
                    (root / "service-host-trust").write_bytes((root / "revoked-service-host-trust.preserves").read_bytes())
                    revoke_current = False
                # READY is not authority. Current reads/verifies the rooted credential itself.
                process.stdin.write("READY " + refs[1] + "\n")
                process.stdin.flush()
                signing_requests.append(refs[1])
            elif line.startswith("RESULT "):
                if result is not None:
                    raise RuntimeError("host emitted more than one terminal result")
                result = json.loads(line[len("RESULT "):])
            else:
                raise RuntimeError("unexpected host stdout frame; refusing implicit signing")
        process.stdin.close()
        exit_code = process.wait(timeout=args.timeout)
        if exit_code != 0 and expect_failure:
            result = {"denied": True, "exit_code": exit_code, "semantic_outcome": "denied"}
        elif (exit_code != 0 and not kill_after_native) or result is None:
            raise RuntimeError("actual host command failed or produced no terminal result; do not retry")
    except BaseException:
        process.kill()
        process.wait()
        raise
    finally:
        deadline.cancel()
    return process.pid, process.returncode, signing_requests, result
