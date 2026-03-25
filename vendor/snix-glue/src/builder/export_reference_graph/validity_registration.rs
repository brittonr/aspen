use nix_compat::store_path::StorePath;

/// Produces the format that Nix calls "makeValidityRegistration" and describes as:
/// "A string accepted by decodeValidPathInfo() that registers the specified paths as valid."
/// Explained differently, this is the text format used in exportReferencesGraph,
/// in the non-structured attrs case, which can be passed to `nix-store --register-validity`.
/// We don't implement the showDerivers and showHash functionality,
/// as that doesn't seem to be used for exportReferenceGraph.
pub fn make_validity_registration<
    W: std::fmt::Write,
    E: Iterator<Item = (StorePath<S1>, R)>,
    R: Iterator<Item = StorePath<S2>> + ExactSizeIterator,
    S1: std::convert::AsRef<str>,
    S2: std::convert::AsRef<str>,
>(
    w: &mut W,
    elems: E,
) -> std::fmt::Result {
    for (store_path, references) in elems {
        let num_references = references.len();
        write!(
            w,
            "{0}\n\n{num_references}\n",
            store_path.to_absolute_path()
        )?;

        for reference in references {
            writeln!(w, "{}", reference.to_absolute_path())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::sync::LazyLock;

    use nix_compat::store_path::{StorePath, StorePathRef};

    use crate::builder::export_reference_graph::make_validity_registration;

    #[derive(serde::Deserialize)]
    #[allow(unused)]
    struct PathData<'a> {
        #[serde(rename = "closureSize")]
        closure_size: u64,

        #[serde(
            rename = "narHash",
            deserialize_with = "nix_compat::nixhash::serde::from_nix_nixbase32_or_sri"
        )]
        nar_sha256: [u8; 32],

        #[serde(rename = "narSize")]
        nar_size: u64,

        #[serde(borrow)]
        pub path: StorePathRef<'a>,

        pub references: Vec<StorePathRef<'a>>,
    }
    static CLOSURE_HELLO: LazyLock<Vec<PathData<'static>>> = LazyLock::new(|| {
        let json_str = r#"
    [
      {
        "closureSize": 2447000,
        "narHash": "sha256:1b34sg5s4padn47z032p8rn0yvp5ji5ls2p9bsc4p7z37w3r6f2h",
        "narSize": 368208,
        "path": "/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8",
        "references": [
          "/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8",
          "/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1"
        ],
        "valid": true
      },
      {
        "closureSize": 197752,
        "narHash": "sha256:0bvlqny00vmrx0x95ai9bcl3qzr0iyy4fwhngng648psr12zff55",
        "narSize": 197752,
        "path": "/nix/store/kbijm6lc9va8xann3cfyam0vczzmwkxj-xgcc-15.2.0-libgcc",
        "references": [],
        "valid": true
      },
      {
        "closureSize": 33294120,
        "narHash": "sha256:16xw6ngrqak6fm91njbgpyq70rjjrmf4y1gcmrwzi199p01lbxcg",
        "narSize": 274640,
        "path": "/nix/store/mi08jhbcjib1i1kgvbd0fxn2yrnzdv4a-hello-2.12.2",
        "references": [
          "/nix/store/mi08jhbcjib1i1kgvbd0fxn2yrnzdv4a-hello-2.12.2",
          "/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47"
        ],
        "valid": true
      },
      {
        "closureSize": 2078792,
        "narHash": "sha256:1vy1c2rfgdgfy5lpgfkd58d0ly7ripfxsx4m99rx434sh3rdy45d",
        "narSize": 2078792,
        "path": "/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1",
        "references": [
          "/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1"
        ],
        "valid": true
      },
      {
        "closureSize": 33019480,
        "narHash": "sha256:1r6rinhxn3a2mrmvdkmakvqvn9cn302yi08vqzwvjjzjjl77fy0k",
        "narSize": 30374728,
        "path": "/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47",
        "references": [
          "/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8",
          "/nix/store/kbijm6lc9va8xann3cfyam0vczzmwkxj-xgcc-15.2.0-libgcc",
          "/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47"
        ],
        "valid": true
      }
    ]
    "#;
        serde_json::from_str::<Vec<PathData<'static>>>(json_str).expect("must deserialize")
    });

    #[test]
    fn test_validity_registration_complex() {
        let mut s = String::new();
        make_validity_registration(
            &mut s,
            CLOSURE_HELLO.iter().map(|elem| {
                (
                    elem.path.to_owned(),
                    elem.references.iter().map(|sp| sp.to_owned()),
                )
            }),
        )
        .expect("must succeed");

        assert_eq!(
            r#"/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8

2
/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8
/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1
/nix/store/kbijm6lc9va8xann3cfyam0vczzmwkxj-xgcc-15.2.0-libgcc

0
/nix/store/mi08jhbcjib1i1kgvbd0fxn2yrnzdv4a-hello-2.12.2

2
/nix/store/mi08jhbcjib1i1kgvbd0fxn2yrnzdv4a-hello-2.12.2
/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47
/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1

1
/nix/store/pkphs076yz5ajnqczzj0588n6miph269-libunistring-1.4.1
/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47

3
/nix/store/d0d9wqmw5saaynfvmszsda3dmh5q82z8-libidn2-2.3.8
/nix/store/kbijm6lc9va8xann3cfyam0vczzmwkxj-xgcc-15.2.0-libgcc
/nix/store/wb6rhpznjfczwlwx23zmdrrw74bayxw4-glibc-2.42-47
"#,
            s
        );
    }

    #[test]
    fn test_validity_registration_simple() {
        let mut s = String::new();

        let sp_libiconv: StorePath<&str> = StorePath::from_absolute_path(
            b"/nix/store/2ilm1l0xy457r1py73n1sxh2c8pc1sq1-libiconv-109",
        )
        .expect("must parse");
        let sp_hello = StorePath::from_absolute_path(
            b"/nix/store/8ha1dhmx807czjczmwy078s4r9s254il-hello-2.12.2",
        )
        .expect("must parse");

        let elems = vec![(&sp_libiconv, [&sp_libiconv]), (&sp_hello, [&sp_libiconv])];

        make_validity_registration(
            &mut s,
            elems
                .as_slice()
                .iter()
                .map(|(path, refs)| ((*path).to_owned(), refs.iter().map(|sp| (*sp).to_owned()))),
        )
        .expect("must succeed");

        assert_eq!(
            r#"/nix/store/2ilm1l0xy457r1py73n1sxh2c8pc1sq1-libiconv-109

1
/nix/store/2ilm1l0xy457r1py73n1sxh2c8pc1sq1-libiconv-109
/nix/store/8ha1dhmx807czjczmwy078s4r9s254il-hello-2.12.2

1
/nix/store/2ilm1l0xy457r1py73n1sxh2c8pc1sq1-libiconv-109
"#,
            s
        );
    }
}
