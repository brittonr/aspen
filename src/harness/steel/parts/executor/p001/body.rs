/// One pending rendering step. Rendering walks the expression tree with an explicit work stack instead
/// of recursion; tasks pop in exactly the order the depth-first walk visits them.
enum RenderTask<'a> {
    /// Admits and renders one expression at a nesting depth.
    Expression(&'a Expr, usize),
    /// Admits and renders one quoted datum at a nesting depth.
    Datum(&'a Expr, usize),
    /// Appends fixed text.
    Text(&'static str),
    /// Leaves a lambda body: unbinds its parameters by truncating the scope to this length.
    Scope(usize),
}

struct Renderer<'a> {
    globals: &'a std::collections::BTreeSet<&'a str>,
    scope: Vec<&'a str>,
    text: String,
    /// Pending work of the current `expression` call.
    tasks: Vec<RenderTask<'a>>,
}

impl<'a> Renderer<'a> {
    fn push_user_identifier(&mut self, name: &str) {
        self.text.push_str(USER_NAMESPACE);
        self.text.push_str(name);
    }

    fn expression(&mut self, expr: &'a Expr, depth: usize) -> std::result::Result<(), String> {
        self.tasks.clear();
        self.tasks.push(RenderTask::Expression(expr, depth));
        while let Some(task) = self.tasks.pop() {
            match task {
                RenderTask::Expression(expr, depth) => self.expression_node(expr, depth)?,
                RenderTask::Datum(expr, depth) => self.datum_node(expr, depth)?,
                RenderTask::Text(text) => self.text.push_str(text),
                RenderTask::Scope(scope_length) => self.scope.truncate(scope_length),
            }
        }
        Ok(())
    }

    /// Admits one expression, renders its leading text, and schedules its subexpressions.
    fn expression_node(&mut self, expr: &'a Expr, depth: usize) -> std::result::Result<(), String> {
        if depth > STEEL_MAX_NESTING {
            return Err(format!("expressions nest deeper than {STEEL_MAX_NESTING}"));
        }
        let next = depth + 1;
        match expr {
            Expr::Atom(atom) => self.atom(&atom.syn.ty),
            Expr::If(branch) => {
                if is_implicit_else(&branch.else_expr) {
                    return Err("if requires an explicit else branch".to_string());
                }
                self.text.push_str("(if ");
                self.tasks.extend([
                    RenderTask::Text(")"),
                    RenderTask::Expression(&branch.else_expr, next),
                    RenderTask::Text(" "),
                    RenderTask::Expression(&branch.then_expr, next),
                    RenderTask::Text(" "),
                    RenderTask::Expression(&branch.test_expr, next),
                ]);
                Ok(())
            }
            Expr::LambdaFunction(function) => self.lambda(function, next),
            Expr::Begin(begin) => {
                if begin.exprs.is_empty() {
                    return Err("begin requires at least one expression".to_string());
                }
                self.text.push_str("(begin ");
                self.schedule_items(&begin.exprs, next, RenderTask::Expression);
                Ok(())
            }
            Expr::Quote(quote) => {
                self.text.push_str("(quote ");
                self.tasks.extend([RenderTask::Text(")"), RenderTask::Datum(&quote.expr, next)]);
                Ok(())
            }
            Expr::List(list) => {
                if list.improper || list.args.is_empty() {
                    return Err("applications must be proper and non-empty".to_string());
                }
                if list.args.len() > STEEL_MAX_ARITY + 1 {
                    return Err(format!("applications pass at most {STEEL_MAX_ARITY} arguments"));
                }
                self.text.push('(');
                self.schedule_items(&list.args, next, RenderTask::Expression);
                Ok(())
            }
            Expr::Define(_) => Err("definitions are admitted only at top level".to_string()),
            Expr::Let(_) => Err("plain-let is not admitted".to_string()),
            Expr::Return(_) => Err("return! is not admitted".to_string()),
            Expr::Macro(_) | Expr::SyntaxRules(_) => Err("macros are not admitted".to_string()),
            Expr::Set(_) => Err("set! is not admitted".to_string()),
            Expr::Require(_) => Err("require is not admitted".to_string()),
            Expr::Vector(_) => Err("vector literals are not admitted".to_string()),
        }
    }

    /// Renders a lambda's metered closure prelude and binds its parameters, then schedules its body
    /// followed by the unbinding of those parameters.
    fn lambda(
        &mut self,
        function: &'a steel::parser::ast::LambdaFunction,
        next: usize,
    ) -> std::result::Result<(), String> {
        if function.rest || function.kwargs {
            return Err("rest and keyword parameters are not admitted".to_string());
        }
        if function.args.len() > STEEL_MAX_ARITY {
            return Err(format!("lambdas take at most {STEEL_MAX_ARITY} parameters"));
        }
        let scope_depth = self.scope.len();
        // Every closure evaluation is charged before the closure escapes: the base cost plus one slot
        // for each variable it can capture (everything in scope) or bind.
        let slots = u64::try_from(scope_depth + function.args.len())
            .map_err(|_| "lambda closure slots exceed the meter's byte range".to_string())?;
        let closure_bytes = STEEL_CLOSURE_BYTES.saturating_add(STEEL_SCALAR_BYTES.saturating_mul(slots));
        self.text.push('(');
        self.text.push_str(CLOSURE);
        self.text.push(' ');
        self.text.push_str(&closure_bytes.to_string());
        self.text.push_str(" (lambda (");
        for (index, parameter) in function.args.iter().enumerate() {
            let name = binder_name(parameter)?;
            if self.scope[scope_depth..].contains(&name) {
                return Err(format!("parameter {name} is bound more than once"));
            }
            if index > 0 {
                self.text.push(' ');
            }
            self.push_user_identifier(name);
            self.scope.push(name);
        }
        self.text.push_str(") (");
        self.text.push_str(FUEL_TICK);
        self.text.push_str(") ");
        self.tasks.extend([
            RenderTask::Text("))"),
            RenderTask::Scope(scope_depth),
            RenderTask::Expression(&function.body, next),
        ]);
        Ok(())
    }

    /// Schedules `items` as `task`s at `depth` followed by a closing bracket, so they render in order with
    /// a space before every item but the first.
    fn schedule_items(&mut self, items: &'a [Expr], depth: usize, task: fn(&'a Expr, usize) -> RenderTask<'a>) {
        self.tasks.reserve(items.len().saturating_mul(2).saturating_add(1));
        self.tasks.push(RenderTask::Text(")"));
        for (index, item) in items.iter().enumerate().rev() {
            self.tasks.push(task(item, depth));
            if index > 0 {
                self.tasks.push(RenderTask::Text(" "));
            }
        }
    }

    fn atom(&mut self, token: &Token) -> std::result::Result<(), String> {
        match token {
            Token::Identifier(name) => {
                let name = name.resolve();
                if self.scope.contains(&name) || self.globals.contains(name) {
                    self.push_user_identifier(name);
                } else if let Some((_, private)) = STEEL_PRIMITIVES.iter().find(|(public, _)| *public == name) {
                    self.text.push_str(private);
                } else {
                    return Err(format!("identifier {name} is not available in the Steel sandbox"));
                }
                Ok(())
            }
            literal => self.literal(literal),
        }
    }

    fn literal(&mut self, token: &Token) -> std::result::Result<(), String> {
        match token {
            Token::StringLiteral(text) => {
                self.text.push('"');
                self.text.push_str(&escape_steel_string(text.resolve()));
                self.text.push('"');
            }
            Token::Number(number) => {
                let rendered = number.to_string();
                let integer = rendered
                    .parse::<isize>()
                    .map_err(|_| format!("number {rendered} is not an admitted fixed-width integer"))?;
                self.text.push_str(&integer.to_string());
            }
            Token::BooleanLiteral(value) => self.text.push_str(if *value { "#true" } else { "#false" }),
            other => return Err(format!("token {other} is not admitted")),
        }
        Ok(())
    }

    /// Admits one quoted datum, renders it, and schedules the items of a quoted list.
    fn datum_node(&mut self, expr: &'a Expr, depth: usize) -> std::result::Result<(), String> {
        if depth > STEEL_MAX_NESTING {
            return Err(format!("quoted data nests deeper than {STEEL_MAX_NESTING}"));
        }
        match expr {
            Expr::Atom(atom) => match &atom.syn.ty {
                Token::Identifier(name) => {
                    let name = name.resolve();
                    validate_identifier(name).map_err(|issue| format!("quoted symbol {issue}"))?;
                    self.text.push_str(name);
                    Ok(())
                }
                literal => self.literal(literal),
            },
            Expr::List(list) if !list.improper => {
                self.text.push('(');
                self.schedule_items(&list.args, depth + 1, RenderTask::Datum);
                Ok(())
            }
            _ => Err("quoted data must be symbols, literals, or proper lists".to_string()),
        }
    }
}

/// The reader fills a missing else branch with a private void primitive; the sandbox requires an
/// explicit branch.
fn is_implicit_else(expr: &Expr) -> bool {
    matches!(expr, Expr::Atom(atom) if matches!(&atom.syn.ty, Token::Identifier(name) if name.resolve().starts_with('#')))
}
