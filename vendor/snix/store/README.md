# //snix/store

This contains the code hosting the snix-store.

For the local store, Nix realizes files on the filesystem in `/nix/store` (and
maintains some metadata in a SQLite database). For "remote stores", it
communicates this metadata in NAR (Nix ARchive) and NARInfo format.

Compared to the Nix model, `snix-store` stores data on a much more granular
level than that, which provides more deduplication possibilities, and more
granular copying.

However, enough information is preserved to still be able to render NAR and
NARInfo when needed.

## More Information
The store consists out of two different gRPC services, `snix.castore.v1` for
the low-level content-addressed bits, and `snix.store.v1` for the Nix and
`StorePath`-specific bits.

Check the `protos/` subfolder both here and in `castore` for the definition of
the exact RPC methods and messages.

## Interacting with the GRPC service manually
The shell environment in `//snix` provides `evans`, which is an interactive
REPL-based gPRC client.

You can use it to connect to a `snix-store` and call the various RPC methods.

```shell
$ cargo run -- daemon &
$ evans --host localhost --port 8000 -r repl
  ______
 |  ____|
 | |__    __   __   __ _   _ __    ___
 |  __|   \ \ / /  / _. | | '_ \  / __|
 | |____   \ V /  | (_| | | | | | \__ \
 |______|   \_/    \__,_| |_| |_| |___/

 more expressive universal gRPC client


localhost:8000> package snix.castore.v1
snix.castore.v1@localhost:8000> service BlobService

snix.castore.v1.BlobService@localhost:8000> call Put --bytes-from-file
data (TYPE_BYTES) => /run/current-system/system
{
  "digest": "KOM3/IHEx7YfInAnlJpAElYezq0Sxn9fRz7xuClwNfA="
}

snix.castore.v1.BlobService@localhost:8000> call Read --bytes-as-base64
digest (TYPE_BYTES) => KOM3/IHEx7YfInAnlJpAElYezq0Sxn9fRz7xuClwNfA=
{
  "data": "eDg2XzY0LWxpbnV4"
}

$ echo eDg2XzY0LWxpbnV4 | base64 -d
x86_64-linux
```

Thanks to `snix-store` providing gRPC Server Reflection (with `reflection`
feature), you don't need to point `evans` to the `.proto` files.
