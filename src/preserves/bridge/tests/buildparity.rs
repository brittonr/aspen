//! Builder parity: the same logical value built through the rail's `IOValue` builders and through
//! `molten_preserves_core::build` has identical canonical bytes and identical `blake3:` refs.
//! The generator is shared with the reader parity property in `viewparity.rs`.

type PackedValue = molten_preserves_core::model::PackedValue;

/// Symbols and record labels: the rail builders take `&'static str` labels.
pub(super) const LABELS: [&str; 8] = ["checks", "check", "some", "none", "refs", "\u{e9}t\u{e9}", "", "a b"];
const CHECK_NAMES: [&str; 3] = ["shape", "policy", "budget"];
const CHECK_STATUSES: [&str; 5] = ["pass", "fail", "deny", "diagnostic", "PASS"];
const MAX_TEXT: usize = 12;
const MAX_ITEMS: usize = 3;
const MAX_LEAVES: usize = 5;
const MAX_COMBINATIONS: usize = 6;

/// One logical value, built both ways.
pub(super) struct Built {
    pub(super) rail: super::IoValue,
    pub(super) kernel: PackedValue,
}

pub(super) fn draw_index(tc: &hegel::TestCase, max: usize) -> usize {
    tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max))
}

fn pick<'a>(tc: &hegel::TestCase, choices: &[&'a str]) -> &'a str {
    choices[draw_index(tc, choices.len() - 1)]
}

/// Canonical refs, near misses (uppercase digest, short digest, missing prefix) and free text.
fn draw_reference(tc: &hegel::TestCase) -> String {
    let canonical = super::content_ref_from_bytes(&tc.draw(hegel::generators::binary().max_size(MAX_TEXT)));
    match draw_index(tc, 4) {
        0 | 1 => canonical,
        2 => canonical.to_ascii_uppercase().replacen("BLAKE3:", "blake3:", 1),
        3 => canonical[..canonical.len() - 1].to_string(),
        _ => tc.draw(hegel::generators::text().max_size(MAX_TEXT)),
    }
}

fn draw_references(tc: &hegel::TestCase) -> Vec<String> {
    (0..draw_index(tc, MAX_ITEMS)).map(|_| draw_reference(tc)).collect()
}

fn draw_checks(tc: &hegel::TestCase) -> Built {
    let pairs: Vec<(&str, &str)> = (0..draw_index(tc, MAX_ITEMS))
        .map(|_| (pick(tc, &CHECK_NAMES), pick(tc, &CHECK_STATUSES)))
        .collect();
    Built {
        rail: super::checks_value(&pairs),
        kernel: molten_preserves_core::build::checks_value(&pairs),
    }
}

fn draw_text_leaf(tc: &hegel::TestCase) -> Built {
    let text = match draw_index(tc, 2) {
        0 => draw_reference(tc),
        1 => pick(tc, &CHECK_STATUSES).to_string(),
        _ => tc.draw(hegel::generators::text().max_size(MAX_TEXT)),
    };
    Built {
        rail: super::string(&text),
        kernel: molten_preserves_core::build::string(&text),
    }
}

fn draw_scalar(tc: &hegel::TestCase) -> Built {
    match draw_index(tc, 2) {
        0 => {
            let label = pick(tc, &LABELS);
            Built {
                rail: super::symbol(label),
                kernel: molten_preserves_core::build::symbol(label),
            }
        }
        1 => {
            let number = tc.draw(hegel::generators::integers::<u64>());
            Built {
                rail: super::u64_value(number),
                kernel: molten_preserves_core::build::u64_value(number),
            }
        }
        _ => {
            let is_set = tc.draw(hegel::generators::booleans());
            Built {
                rail: super::bool_value(is_set),
                kernel: molten_preserves_core::build::bool_value(is_set),
            }
        }
    }
}

fn draw_refs_leaf(tc: &hegel::TestCase) -> Built {
    if tc.draw(hegel::generators::booleans()) {
        let refs = draw_references(tc);
        return Built {
            rail: super::refs_sequence(&refs),
            kernel: molten_preserves_core::build::refs_sequence(&refs),
        };
    }
    let reference = tc.draw(hegel::generators::booleans()).then(|| draw_reference(tc));
    Built {
        rail: super::optional_ref_value(reference.as_deref()),
        kernel: molten_preserves_core::build::optional_ref_value(reference.as_deref()),
    }
}

fn draw_leaf(tc: &hegel::TestCase) -> Built {
    match draw_index(tc, 3) {
        0 => draw_scalar(tc),
        1 => draw_refs_leaf(tc),
        2 => draw_checks(tc),
        _ => draw_text_leaf(tc),
    }
}

/// Wraps the last few pool entries in a sequence or a symbol-labelled record.
fn combine(tc: &hegel::TestCase, pool: &mut Vec<Built>) {
    let items = pool.split_off(pool.len().saturating_sub(draw_index(tc, MAX_ITEMS)));
    let (rail, kernel): (Vec<_>, Vec<_>) = items.into_iter().map(|built| (built.rail, built.kernel)).unzip();
    let built = if tc.draw(hegel::generators::booleans()) {
        Built {
            rail: super::sequence(rail),
            kernel: molten_preserves_core::build::sequence(kernel),
        }
    } else {
        let label = pick(tc, &LABELS);
        Built {
            rail: super::record(label, rail),
            kernel: molten_preserves_core::build::record(label, kernel),
        }
    };
    pool.push(built);
}

/// A value built bottom-up from a pool of leaves, so the generator never recurses.
pub(super) fn draw_built(tc: &hegel::TestCase) -> Built {
    let mut pool: Vec<Built> = (0..=draw_index(tc, MAX_LEAVES)).map(|_| draw_leaf(tc)).collect();
    for _ in 0..draw_index(tc, MAX_COMBINATIONS) {
        combine(tc, &mut pool);
    }
    if pool.len() == 1 {
        return pool.remove(0);
    }
    let (rail, kernel) = pool.into_iter().map(|built| (built.rail, built.kernel)).unzip();
    Built {
        rail: super::sequence(rail),
        kernel: molten_preserves_core::build::sequence(kernel),
    }
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
// r[verify molten.preserves_rail_toolkit.parser_builders]
#[hegel::test(test_cases = 300)]
fn hegel_kernel_builders_match_rail_bytes_and_refs(tc: hegel::TestCase) {
    let built = draw_built(&tc);
    let rail_bytes = super::canonical_bytes(&built.rail).expect("rail canonical bytes");
    assert_eq!(molten_preserves_core::writer::encode_canonical(&built.kernel), rail_bytes);
    let kernel_hash = molten_preserves_core::build::canonical_hash(&built.kernel);
    assert_eq!(kernel_hash, super::canonical_hash(&built.rail).expect("rail hash"));
    let kernel_ref = molten_preserves_core::build::content_ref_from_bytes(&rail_bytes);
    assert_eq!(kernel_ref, super::content_ref_from_bytes(&rail_bytes));
    assert_eq!(super::packed_from_iovalue(&built.rail).expect("bridge conversion"), built.kernel);
    assert_eq!(super::iovalue_from_packed(&built.kernel).expect("bridge conversion"), built.rail);
}
