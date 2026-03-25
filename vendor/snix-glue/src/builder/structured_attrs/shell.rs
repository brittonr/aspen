//! Contains the code rendering the shell script that's used for structured attrs.
//!

use regex::Regex;
use std::sync::LazyLock;

static RE_SH_VAR_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap());

/// Writes an escaped version of the passed string to the writer.
fn write_shell_escaped_single_quoted<W>(f: &mut W, s: &str) -> std::fmt::Result
where
    W: std::fmt::Write,
{
    write!(f, "'")?;
    for c in s.chars() {
        if c == '\'' {
            write!(f, "\\'")?;
        } else {
            write!(f, "{c}")?;
        }
    }
    write!(f, "'")?;
    Ok(())
}

/// determine if the value is "good to print".
/// We essentially want to reject floats which are not just integers.
fn is_good_simple_value(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => true,
        serde_json::Value::Number(number) => {
            if number.as_i64().is_some() || number.as_u64().is_some() {
                true
            } else if let Some(n) = number.as_f64() {
                n.ceil() == n
            } else if number.as_i128().is_some() {
                true
            } else {
                number.as_u128().is_some()
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            unreachable!("Snix bug: called write_simple_type on complex type")
        }
    }
}

fn write_simple_type<W>(f: &mut W, v: serde_json::Value) -> std::fmt::Result
where
    W: std::fmt::Write,
{
    match v {
        serde_json::Value::Null => write!(f, "''")?,
        serde_json::Value::Bool(v) => {
            if v {
                write!(f, "1")?;
            } else {
                write!(f, "")?;
            }
        }
        serde_json::Value::Number(number) => {
            if let Some(n) = number.as_i64() {
                write!(f, "{n}")?;
            } else if let Some(n) = number.as_u64() {
                write!(f, "{n}")?;
            } else if let Some(n) = number.as_f64() {
                debug_assert!(n.ceil() == n, "bad number value");
                write!(f, "{}", n.ceil() as i64)?;
            } else if let Some(n) = number.as_i128() {
                write!(f, "{n}")?;
            } else if let Some(n) = number.as_u128() {
                write!(f, "{n}")?;
            } else {
                panic!("unable to represent number");
            }
        }
        serde_json::Value::String(s) => {
            write_shell_escaped_single_quoted(f, &s)?;
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            unreachable!("Snix bug: called write_simple_type on complex type")
        }
    }

    Ok(())
}

/// for a given json map, write the file contents of the to-be-sourced bash script.
/// Cf. writeStructuredAttrsShell in Cppnix.
pub fn write_attrs_sh_file<W>(
    f: &mut W,
    map: serde_json::Map<String, serde_json::Value>,
) -> std::fmt::Result
where
    W: std::fmt::Write,
{
    for (k, v) in map {
        // keys with spaces, backslashes (and potentially everything else making
        // that key an invalid identifier) are silently skipped from the bash
        // file (but present in the ATerm!)
        if !RE_SH_VAR_NAME.is_match(k.as_str()) {
            continue;
        }
        match v {
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {
                write!(f, "declare {k}=")?;
                write_simple_type(f, v)?;
                writeln!(f)?;
            }
            serde_json::Value::Number(_) => {
                // Nix skips over bad values (floats which can't be represented as integers)
                if !is_good_simple_value(&v) {
                    continue;
                }
                write!(f, "declare {k}=")?;
                write_simple_type(f, v)?;
                writeln!(f)?;
            }
            serde_json::Value::Array(values) => {
                // If the array contains any invalid element, the array is skipped entirely.
                if values.iter().any(|v| !is_good_simple_value(v)) {
                    continue;
                }
                write!(f, "declare -a {k}=(")?;
                for val in values {
                    write_simple_type(f, val)?;
                    write!(f, " ")?;
                }
                writeln!(f, ")")?;
            }
            serde_json::Value::Object(map) => {
                // If the array contains any invalid element, or another
                // non-simple type, the object is skipped entirely.
                if map.values().any(|v| {
                    matches!(
                        v,
                        serde_json::Value::Array(_) | serde_json::Value::Object(_)
                    ) || !is_good_simple_value(v)
                }) {
                    continue;
                }

                write!(f, "declare -A {k}=(")?;
                for (k, v) in map {
                    write!(f, "['{k}']=")?;
                    write_simple_type(f, v)?;
                    write!(f, " ")?;
                }
                writeln!(f, ")")?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use rstest::rstest;
    use serde_json::json;

    use super::write_attrs_sh_file;

    #[rstest]
    #[case::empty(json!({}), "")]
    #[case::empty_key(json!({"": "value"}), "")]
    #[case::null(json!({"k": null}), r#"declare k=''"#)]
    #[case::string(json!({"k":"v"}), r#"declare k='v'"#)]
    #[case::string_escaping(json!({"k":"v'"}), r#"declare k='v\''"#)]
    #[case::bool_false(json!({"k":false}), r#"declare k="#)]
    #[case::bool_true(json!({"k":true}), r#"declare k=1"#)]
    #[case::number(json!({"k":1}), r#"declare k=1"#)]
    #[case::number_float(json!({"k":1.0}), r#"declare k=1"#)]
    #[case::number_float_invalid(json!({"k":1.1}), r#""#)]
    #[case::array_of_strings(json!({"k": ["bar", "baz"]}), r#"declare -a k=('bar' 'baz' )"#)]
    #[case::array_of_strings_and_bool(json!({"k": ["bar", true]}), r#"declare -a k=('bar' 1 )"#)]
    #[case::array_of_strings_and_invalid_number(json!({"k": ["bar", 1.1]}), "")]
    #[case::object(json!(
        {"k": {
            "bar": true,
            "b": 1.0,
            "c": false,
            "d": true,
        }}), r#"declare -A k=(['b']=1 ['bar']=1 ['c']= ['d']=1 )"#)]
    #[case::object_invalid_number(json!(
        {"k": {
            "bar": true,
            "b": 1.1,
        }}), "")]
    #[case::object_too_complex(json!(
        {"k": {
            "bar": true,
            "baz": [],
        }}), r#""#)]
    #[case::multiple(json!(
        {
            "k": {
                "bar": true,
                "b": 1.0,
                "c": false,
                "d": true,
            },
            "l": 42,
            "m": false,
            "n": 1.1,
        }),
        r#"declare -A k=(['b']=1 ['bar']=1 ['c']= ['d']=1 )
declare l=42
declare m="#)]
    fn write_attrs(#[case] val: serde_json::Value, #[case] exp_output: &str) {
        let mut out = String::new();
        let map = val.as_object().expect("must be map").to_owned();
        write_attrs_sh_file(&mut out, map).expect("must succeed");

        if exp_output.is_empty() {
            assert_eq!(exp_output, out, "expected output to match");
        } else {
            assert_eq!(
                {
                    let mut exp_output = String::from(exp_output);
                    exp_output.push('\n');
                    exp_output
                },
                out,
                "expected output to match"
            );
        }
    }
}
