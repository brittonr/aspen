# Fresh signed source baseline (2026-11-21)

## Purpose

Rebuild and sign a source output whose measured closure matches the actual store,
then re-run the real three-process collaboration driver. This removes the stale
signed-output blocker recorded during the 2026-11-20 guest attempt.

## Fresh output

Built locally with:

```
nix build .#molten-startup-source-binding --no-update-lock-file --option builders ''
```

Result:

```
/nix/store/9kh84csrlczijhwr04gg0xkvg0dxrl9y-molten-node-source-binding-v1
```

The output was signed with an experimental local key
(`molten-guest-20261120`). The matching public key was written into the
operator-provisioned trust record; no secret material is retained here.

Selection refs:

- binding `blake3:9b7f6d59cd1b573800ee074701c11222563691308ce4dc23433f69b724133949`
- policy `blake3:e045c1f58ded52d1d1454096b8fc7618cebab06fd4b7b250c1be93a88bce394c`
- profile `blake3:902c6db77c69d2d616b95cce54adbf692e886de8fcaa372ae4f24225ee48aa9d`

## Admission

`molten node init` completed, then `molten node run` authenticated the full
build closure and published:

```
node run startup=blake3:de93d08cc73c09e4c151f3526a9eecd83a45ea7975684004b0b688c98b7daa48 adapters=12
```

## Driver baseline

The real collaboration driver ran with `MOLTEN_TEST_SOURCE_TRUST` selecting the
fresh trust record. It completed the baseline three-node flow and retained:

```
baseline observations=blake3:de2c103b33c9e0420972903386822e2477450bfe1f1d5dc4d5e3ecc19707434a
```

## Boundary

The driver process was stopped after the baseline observation so it did not run
the six additional fault/boundary cases. This is host execution with the fresh
signed output; it is not guest, KVM, campaign, replay, or production
qualification. The guest cohort task remains open.
