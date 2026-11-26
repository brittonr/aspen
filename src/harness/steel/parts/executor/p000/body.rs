/// Admits `source` against the closed sandbox language and renders the instrumented program text,
/// plus the private global name of `callable`. The callable is validated as an identifier and
/// resolved as a top-level definition; it is never spliced into program text.
fn compile_program(source: &str, callable: &str) -> std::result::Result<(String, String), String> {
    validate_identifier(callable).map_err(|issue| format!("callable {issue}"))?;
    validate_bracket_depth(source)?;
    let forms = Engine::emit_ast(source).map_err(|error| format!("source does not parse: {error}"))?;
    if forms.is_empty() {
        return Err("source defines nothing".to_string());
    }
    if forms.len() > STEEL_MAX_DEFINITIONS {
        return Err(format!("source has more than {STEEL_MAX_DEFINITIONS} top-level definitions"));
    }
    let mut definitions = Vec::with_capacity(forms.len());
    let mut globals = std::collections::BTreeSet::new();
    for form in &forms {
        let Expr::Define(define) = form else {
            return Err("only top-level definitions are admitted".to_string());
        };
        let name = binder_name(&define.name)?;
        if !globals.insert(name) {
            return Err(format!("{name} is defined more than once"));
        }
        definitions.push((name, &define.body));
    }
    if !globals.contains(callable) {
        return Err(format!("callable {callable} is not a top-level definition"));
    }
    let mut renderer = Renderer {
        globals: &globals,
        scope: Vec::new(),
        text: String::with_capacity(source.len().saturating_mul(2)),
        tasks: Vec::new(),
    };
    for (name, body) in definitions {
        renderer.text.push_str("(define ");
        renderer.push_user_identifier(name);
        renderer.text.push(' ');
        renderer.expression(body, 0)?;
        renderer.text.push_str(")\n");
    }
    Ok((renderer.text, format!("{USER_NAMESPACE}{callable}")))
}

/// Rejects bracket nesting deeper than the reviewed grammar admits before Steel's recursive reader
/// runs. Brackets inside string literals, comments, and character literals do not count.
fn validate_bracket_depth(source: &str) -> std::result::Result<(), String> {
    let mut depth = 0_usize;
    let mut characters = source.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => {
                while let Some(inner) = characters.next() {
                    match inner {
                        '\\' => {
                            characters.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            ';' => {
                for inner in characters.by_ref() {
                    if inner == '\n' {
                        break;
                    }
                }
            }
            '#' if characters.clone().next() == Some('\\') => {
                characters.next();
                characters.next();
            }
            '(' | '[' | '{' => {
                depth += 1;
                if depth > STEEL_MAX_BRACKET_DEPTH {
                    return Err(format!("brackets nest deeper than {STEEL_MAX_BRACKET_DEPTH}"));
                }
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Admits a binding name: a plain identifier from a closed character set. Collisions with
/// primitives or VM names are impossible because every binding is renamed into `USER_NAMESPACE`.
fn validate_identifier(name: &str) -> std::result::Result<(), String> {
    let Some(first) = name.chars().next() else {
        return Err("is empty".to_string());
    };
    if name.len() > STEEL_MAX_IDENTIFIER_BYTES {
        return Err(format!("exceeds {STEEL_MAX_IDENTIFIER_BYTES} bytes"));
    }
    if !first.is_ascii_alphabetic() {
        return Err(format!("{name:?} must start with an ASCII letter"));
    }
    if !name.chars().all(|character| character.is_ascii_alphanumeric() || "-_?!*<>=/".contains(character)) {
        return Err(format!("{name:?} contains characters outside the reviewed identifier set"));
    }
    Ok(())
}

fn binder_name(expr: &Expr) -> std::result::Result<&str, String> {
    let Expr::Atom(atom) = expr else {
        return Err("definition and parameter names must be identifiers".to_string());
    };
    let Token::Identifier(name) = &atom.syn.ty else {
        return Err("definition and parameter names must be identifiers".to_string());
    };
    let name = name.resolve();
    validate_identifier(name).map_err(|issue| format!("binding {issue}"))?;
    Ok(name)
}
