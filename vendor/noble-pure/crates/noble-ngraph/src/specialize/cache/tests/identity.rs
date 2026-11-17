use super::*;

fn unhex(text: &str) -> alloc::vec::Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("hex high digit");
            let low = (pair[1] as char).to_digit(16).expect("hex low digit");
            u8::try_from(high * 16 + low).expect("one byte")
        })
        .collect()
}

#[test]
fn shared_length_prefixed_encoding_matches_both_pinned_fixtures() {
    let specialization_parts: [&[u8]; 7] = [
        b"ngraph-ir/0 ngraph-i64/0 graph bytes",
        b"original-interface=i64,i64",
        b"static-map=0:i64:7,1:dynamic",
        b"ngraph-sem/0",
        b"ngraph-facts/0",
        b"ngraph-pe/0",
        b"budgets=default/0",
    ];
    let specialization = encode_parts(
        b"noble.ngraph.specialization-key.v1",
        &specialization_parts,
        usize::MAX,
    )
    .expect("fixture encoding");
    assert_eq!(
        specialization,
        unhex(concat!(
            "6e6f626c652e6e67726170682e7370656369616c697a6174696f6e2d6b65792e7631",
            "24000000000000006e67726170682d69722f30206e67726170682d6936342f30206772617068206279746573",
            "1a000000000000006f726967696e616c2d696e746572666163653d6936342c693634",
            "1c000000000000007374617469632d6d61703d303a6936343a372c313a64796e616d6963",
            "0c000000000000006e67726170682d73656d2f30",
            "0e000000000000006e67726170682d66616374732f30",
            "0b000000000000006e67726170682d70652f30",
            "1100000000000000627564676574733d64656661756c742f30"
        ))
    );

    let build_parts: [&[u8]; 8] = [
        b"residual graph bytes",
        b"target=noble-core-v1",
        b"representation=declared-text-v1",
        b"backend=noble-kernel/0",
        b"flags=default-limits-and-budgets/0",
        b"runtime=noble-core/0",
        b"abi=i64->i64",
        b"compiler=ngraph-layered/0",
    ];
    let build = encode_parts(
        b"noble.ngraph.program-build-key.v1",
        &build_parts,
        usize::MAX,
    )
    .expect("fixture encoding");
    assert_eq!(
        build,
        unhex(concat!(
            "6e6f626c652e6e67726170682e70726f6772616d2d6275696c642d6b65792e7631",
            "1400000000000000726573696475616c206772617068206279746573",
            "14000000000000007461726765743d6e6f626c652d636f72652d7631",
            "1f00000000000000726570726573656e746174696f6e3d6465636c617265642d746578742d7631",
            "16000000000000006261636b656e643d6e6f626c652d6b65726e656c2f30",
            "2200000000000000666c6167733d64656661756c742d6c696d6974732d616e642d627564676574732f30",
            "140000000000000072756e74696d653d6e6f626c652d636f72652f30",
            "0c000000000000006162693d6936342d3e693634",
            "1900000000000000636f6d70696c65723d6e67726170682d6c6179657265642f30"
        ))
    );

    let model_parts: [&[u8]; 3] = [b"specialization-cache-fixture", b"v1", b"bytes"];
    let rust_encoding = encode_parts(b"noble.ngraph.subject.v0", &model_parts, usize::MAX)
        .expect("shared Lean/Rust encoding");
    assert_eq!(
        rust_encoding,
        unhex(concat!(
            "6e6f626c652e6e67726170682e7375626a6563742e7630",
            "1c000000000000007370656369616c697a6174696f6e2d63616368652d66697874757265",
            "02000000000000007631",
            "05000000000000006279746573"
        ))
    );
}
