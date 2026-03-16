# Introduction to Dynamic Derivations

> Source: https://github.com/pdtpartners/nix-ninja/blob/main/docs/dynamic-derivations.md
> Retrieved: 2026-03-16

Currently, Nix builds at package-granularity, which means each package is built
from scratch every time, though you can skip packages that have already been
built.

Dynamic derivations is a set of features that essentially allows the unit of
build in Nix "derivations" to be able to generate more derivations at build-time.
This enables IFD-less lang2nix implementations and incremental compilation in
Nix.

It's been in the works for a while, and finally at the point where we can start
experimenting.

> This document distills the docs and source code into a condensed
writeup to help you build tooling that leverages dynamic derivations.

Sources I used:

- [Cpp Nix source](https://github.com/NixOS/nix/tree/master/src)
- [Nix manual](https://github.com/NixOS/nix/tree/master/doc/manual/source)
- [Sandstone - incremental haskell builds with dynamic derivations](https://github.com/obsidiansystems/sandstone)
- [Haskell Nix source in Obsidian System's fork](https://github.com/obsidiansystems/hnix-store/tree/derivation-work)
- [Tvix nix-compat crate source](https://code.tvl.fyi/tree/tvix/nix-compat)

In order to have effective incremental compilation in Nix, we need these
experimental features enabled:

- `dynamic-derivations` allows derivations whose inputs depend on outputs of
  other derivations at build time.
- `ca-derivations` ensures identical outputs have same store path regardless of
  inputs, which is crucial for incremental compilation

At a high-level, this is all made possible because two new features:

- New `text` output hash mode that allows you to write an ATerm serialized
  derivation into `$out`.
- New `nix-computed-output` "placeholder" that Nix knows how to fill at
  build-time and when decoded by Nix, it knows which derivation output to use

We'll get to them later, but first there's some necessary context to go
through.

# Content-addressed derivations

A derivation can be one of three types:

1. Input-addressed derivations (default)
2. Fixed-output derivations (FODs)
3. Floating content-addressed derivations

(3) requires experimental feature `ca-derivations` to be enabled to use.

A derivation is considered (3), ca derivation for short, if:

- `__contentAddressed = true`
- `outputHashAlgo` and `outputHashMode` are set
- `outputHash` is NOT set

Notably, FODs (2) has content hashes in `outputHash` but is NOT
content-addressed because the "address" part refers to how it's referenced,
i.e. FODs are referenced by input-addressed Nix store paths.

> Side thought:
> If FODs were wrapped by (3) ca derivations, then it can be cached across Nix
stores with different nix store prefixes like `/opt/store` and also survive
unnecessary rebuild-the-worlds when inputs like `curl` changes.

Moving on, the `outputHashMode` has four possible values:

- `flat` (default) for a single non-executable file
- `recursive` or `nar` for directories
- `text` used for `dynamic-derivations` experimental feature
- `git` used for `git-hashing` experimental feature

FODs typically use the `flat` mode which doesn't allow references to other
Nix store objects. However, the new `text` mode does allow references except
self-references. The hash mode `text` is used for `.drv` outputs which Nix
will continue building directly without IFD.

# ATerm file format

ATerm is an existing data format for representing tree-like structures (similar
to XML or JSON). Derivation files `.drv` are stored in the Nix store in ATerm
format.

They must be serialized with a top-level:

- `Derive(...)`
- `DrvWithVersion(<version-string>, ...)`

The only accepted `version-string` today is `xp-dyn-drv` for the
`dynamic-derivations` experimental feature.

From reading `NixOS/nix` source code, it's considered the cleaner to use
`DrvWithVersion` when leveraging `dynamic-derivations` features but it doesn't
seem strictly necessary. It's only purpose is to check whether the current Nix
daemon is compatible with the `DrvWithVersion(...)` on disk. E.g. You
previously had `dynamic-derivations` enabled but not anymore, or you copied
over from a Nix store that had `dynamic-derivations` but you don't.

I would recommend using JSON derivation format for simplicity. Just add it to
the derivation and let Nix serialize it into ATerm for you, then copy from that
`.drv` store path to `$out`.

```json
{
  "name": "...",
  "system": "x86_64-linux",
  "args": [],
  "builder": "...",
  "env": {},
  "inputDrvs": {
    "/nix/store/<hash>-<name>.drv": {
      "dynamicOutputs": {},
      "outputs": [
        "out"
      ]
    }
  },
  "inputSrcs": [
    "/nix/store/<hash>-<name>"
  ],
  "outputs": {
    "out": {
      "hashAlgo": "sha256",
      "method": "nar"
    }
  }
}
```

# JSON derivation format deep-dive

A derivation consists of:

- A name
- The `system` type (e.g. `x86_64-linux`) where the executable is to run
- An inputs spec
- An outputs spec
- The process creation fields for the build process

There are two types of inputs:

- `inputSrcs` is an array of store paths, and Nix will make them available
  to the build process
- `inputDrvs` is an array of "output deriving paths", which are structured
  objects that describe drv path + output name(s).

For example:

```json
{
  "inputSrcs": [
    "/nix/store/<hash>-<name>"
  ],
  "inputDrvs": {
    "/nix/store/<hash>-<name>.drv": {
      "dynamicOutputs": {},
      "outputs": ["out"]
    }
  }
}
```

Since `inputSrcs` are just store paths, you can just refer to them by absolute
paths in your build process, or use env vars like nixpkgs' `stdenv.mkDerivation`
which sets `$src`.

Whereas `inputDrvs` you must reference them using "placeholders" which are
encoded values that point to outputs of `inputsDrvs`. More on placeholders in
the next section.

Finally, the process creation fields describe how to spawn the process which
will perform the build. It consists of:

- `builder`
- `args`
- `env`

Where `builder` is path to build process executable, `args` is a list of args
and `env` is a dict of environment variables.

# Placeholders

For inputs that are outputs of other derivations, you can reference them in
process creation fields via "placeholders". These are opaque values in the form
of `/<hash>`.

Note that placeholders existed before dynamic derivations:

```sh
nix-repl> builtins.placeholder "foo"
"/1x0ymrsy7yr7i9wdsqy9khmzc1yy7nvxw6rdp72yzn50285s67j5"
```

Under the hood, it's computed with this pseudo code:

```python
# For regular derivations
def placeholder(output_name: str) -> str:
    clear_text = f"nix-output:{output_name}"
    digest = sha256sum(clear_text)
    return nixbase32.encode(digest)
```

This is useful if you want to set an env var to eventually what the output path
is. If you search for `builtins.placeholder` in nixpkgs, you'll find many
occurences:

```nix
KMODDIR = "${builtins.placeholder "out"}/kernel";
```

There are also new placeholders:

- `nix-upstream-output:` for content-addressed derivations
- `nix-computed-output:` for dynamic derivations

They are computed differently (again psuedo-code):

```python
# For content-addressed derivations
def unknown_ca_output(drv_path: str, output_name: str) -> str:
    drv_name = drv_path.removesuffix('.drv')
    clear_text = f"nix-upstream-output:{drv_path.hash_part}:{drv_name}-{output_name}"
    digest = sha256sum(clear_text)
    return nixbase32.encode(digest)

# For dynamic derivations
def unknown_derivation(placeholder: str, output_name: str) -> str:
    # Take first 20 bytes of the input placeholder hash
    compressed = placeholder[:20]
    clear_text = f"nix-computed-output:{compressed}:{output_name}"
    digest = sha256sum(clear_text)
    return nixbase32.encode(digest)
```

# Dynamic outputs

When you depend on derivation-producing derivations, you need to use
`dynamicOutputs` to trigger the code path that handles dynamic derivations.

```json
{
  "inputDrvs": {
    "/nix/store/<hash>-<name>.drv": {
      "dynamicOutputs": {
        "drv-out": {
          "dynamicOutputs": {},
          "outputs": ["out"]
        }
      },
      "outputs": []
    }
  }
}
```

This nested structure allows you to describe the output of derivation which
was generated by another derivation. Here's how you would create a placeholder
that references this dynamic output:

```python
# First get placeholder for "drv-out" output of the original derivation
drv_out_placeholder = unknown_ca_output("/nix/store/<hash>.drv", "drv-out")

# Then get placeholder for "out" output of the produced derivation
out_placeholder = unknown_derivation(drv_out_ph, "out")
```

# New Nix builtins

You can also use ca derivations and dynamic derivations in `.nix` files but I'd
recommend generating JSON derivation format directly to avoid the overhead of
Nix evaluation. Nevertheless, I'll go over the new builtins for completeness:

- `builtins.outputOf` returns a placeholder that references a output path of a
  derivation.
- `builtins.unsafeDiscardOutputDependency` is a leaky implementation detail
  that strips internal string metadata that refers to its output dependencies.

Let's walk through an example:

```nix
{ pkgs ? import <nixpkgs> {} }:

let
  caDrv = pkgs.stdenv.mkDerivation {
    name = "ca-example";
    __contentAddressed = true;
    outputHashMode = "nar";
    outputHashAlgo = "sha256";
    outputs = [ "out" ];
    buildCommand = "...";
  };

  dynDrv = pkgs.stdenv.mkDerivation {
    name = "dynamic-example";
    buildCommand = ''
      ${builtins.outputOf (builtins.unsafeDiscardOutputDependency caDrv) "out"}
    '';
  };

in { inherit caDrv dynDrv; }
```

Ideally the UX is `builtins.outputOf caDrv "out"` but I'll get into why the
other builtin is necessary later.

Let's first look at their JSON representations:

```json
{
  "/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv": {
    "name": "ca-example",
    "env": {
      "out": "/1rz4g4znpzjwh1xymhjpm42vipw92pr73vdgl6xs1hycac8kf2n9"
    },
    "outputs": {
      "out": {
        "hashAlgo": "sha256",
        "method": "nar"
      }
    }
  }
}
```

In the `ca-example.drv`, `$out` is a placeholder value that Nix will fill at
build-time, but you can use it regularly like `mkdir -p $out/bin`, etc.

```json
{
  "/nix/store/b7pcfk2d7knx76jjkb48hipywrkj0aak-dynamic-example.drv": {
    "name": "dynamic-example",
    "env": {
      "buildCommand": "/0g9wr256l3563hj4ivphq5wkyz7kby9h9sx17360q7hjaxjnvqj2\n"
    },
    "inputDrvs": {
      "/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv": {
        "dynamicOutputs": {
          "out": {
            "dynamicOutputs": {},
            "outputs": ["out"]
          }
        },
        "outputs": []
      }
    }
  }
}
```

In the `dynamic-example.drv`, the `buildCommand` gets a `nix-computed-output`
placeholder based on the `dynamicOutputs` of the `ca-example.drv`.

Going back to `builtins.unsafeDiscardOutputDependency`, we can explore how it
works in the Nix repl:

```sh
nix-repl> caDrv
«derivation /nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv»

nix-repl> builtins.outputOf caDrv "out"
error:
       … while calling the 'outputOf' builtin
         at «string»:1:1:
            1| builtins.outputOf caDrv "out"
             | ^

       … while evaluating the first argument to builtins.outputOf

       error: expected a string but found a set
```

What's going on? Turns out you must provide a string, here's the excerpt
from Nix's functional tests:

```bash
# We currently require a string to be passed, rather than a derivation
# object that could be coerced to a string. We might liberalise this in
# the future so it does work, but there are some design questions to
```

Okay let's try `caDrv.drvPath`:

```sh
nix-repl> builtins.outputOf caDrv.drvPath "out"
error:
       … while calling the 'outputOf' builtin
         at «string»:1:1:
            1| builtins.outputOf caDrv.drvPath "out"
             | ^

       … while evaluating the first argument to builtins.outputOf

       error: string '/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv'
              has a context which refers to a complete source and binary closure.
              This is not supported at this time.
```

I didn't understand what this meant, but using
`builtins.unsafeDiscardOutputDependency` does fixes it issue, so let's a take
a look at that:

```sh
nix-repl> caDrv.drvPath
"/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv"

nix-repl> builtins.unsafeDiscardOutputDependency caDrv.drvPath
"/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv"
```

Huh? This is getting deep into the weeds, but strings in Nix have a "string
context" which holds metadata. `caDrv.drvPath` has a `DrvDeep` string context
that includes its entire build closure, which `builtins.outputOf` isn't happy
with.

```cpp
/**
 * Path to a derivation and its entire build closure.
 *
 * The path doesn't just refer to derivation itself and its closure, but
 * also all outputs of all derivations in that closure (including the
 * root derivation).
 *
 * Encoded in the form `=<drvPath>`.
 */
struct DrvDeep {
  /* ... */
}
```

`DrvDeep` string contexts are not supported by `builtins.outputOf` at the time
of writing this, but the source code does indicate that it may relax this
requirement in the future.

Anyway, you can explore the inner details by using `builtins.getContext`:

```sh
nix-repl> builtins.toJSON (builtins.getContext caDrv.drvPath)
"{\"/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv\":{\"allOutputs\":true}}"

nix-repl> builtins.toJSON (builtins.getContext (builtins.unsafeDiscardOutputDependency caDrv))
"{\"/nix/store/w283xjf1174klb924fg0b6y5iwlhw1v0-ca-example.drv\":{\"outputs\":[\"out\"]}}"
```

Internally, `"allOutputs": true` indicates a complete closure. After using
`builtins.unsafeDiscardOutputDependency`, it simplifies the context to just
the output. This is just a leaky implementation constraint where
`builtins.outputOf` needs clean derivation path references without full closure
information.

# Command-line dynamic outputs

Finally, dynamic derivations brings a syntax to express dynamic outputs on the
command-line.

```md
/nix/store/<hash>-<name>.drv^foo.drv^bar.drv^out
|------------------------------------------| |-|
inner deriving path                          output name
|----------------------------------| |-----|
even more inner deriving path        output name
|--------------------------| |-----|
innermost store path         output name
```

This is represented by the equivalent `dynamicOutputs`:

```json
{
  "inputDrvs": {
    "/nix/store/<hash>-<name>.drv": {
      "dynamicOutputs": {
        "foo.drv": {
          "dynamicOutputs": {
            "bar.drv": {
              "dynamicOutputs": {},
              "outputs": ["out"]
            },
            "outputs": []
          },
          "outputs": []
        }
      },
      "outputs": []
    }
  }
}
```

And it is supported by `nix build` like so:

```sh
nix build "/nix/store/<hash>-<name>.drv^foo.drv^bar.drv^out"
```

# Conclusion

That's it! As far as I understand these are all the practical elements to
building using dynamic derivations. Let's summarize the main take-aways:

- Incremental compilation requires `ca-derivations` and `dynamic-derivations`
  experimental features
- The `text` hash mode allows a derivation to output a derivation.
- Derivations are traditionally serialized in ATerm format but I recommend
  utilizing the new JSON derivation format that can be written as an output.
- Placeholders are encoded values that reference `dynamicOutputs`
- `dynamicOutputs` is a structured object in `inputDrvs` of a derivaton that can
  describe outputs of a derivation produced by another derivation.
- `builtins.outputOf` has quirks like `builtins.unsafeDiscardOutputDependency`
  to be aware of, but is used at eval-time to produce placeholders.
