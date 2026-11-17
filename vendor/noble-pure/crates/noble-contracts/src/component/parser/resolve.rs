mod declarations;
mod signature;
mod traversal;
mod types;

const MAX_FLAT_PARAMETERS: usize = 16;
const BOOTSTRAP_DEFINITIONS: usize = 24;
const BOOTSTRAP_EFFECTS: usize = 2;

#[derive(Clone, Copy)]
struct Context<'a> {
    package: &'a super::Package,
    selected: &'a super::RawWorld,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; selection, unique names, resolved resource scopes, operation limits and independently reconstructed import contracts are checked fallibly before the owned world is returned."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
pub(super) fn world(
    package: super::Package,
    wit: &[u8],
    selected: &str,
    limits: crate::Limits,
    checked_u64: bool,
) -> Result<crate::component::World, crate::component::Error> {
    attempt!(declarations::unique(&package));
    let mut selected_index = None;
    let mut at = 0usize;
    while at < package.worlds.len() {
        if traversal::is_selected(&package, at, selected) {
            selected_index = Some(at);
        }
        at = at.saturating_add(1);
    }
    let index = match selected_index {
        Some(index) => index,
        None => return Err(crate::component::invalid("selected WIT world is absent")),
    };
    let selected = match package.worlds.get(index) {
        Some(world) => world,
        None => return Err(crate::component::invalid("selected WIT world is absent")),
    };
    let resources = attempt!(declarations::resources(&package));
    let mut world = crate::component::World {
        identity: qualified(&package, &selected.name),
        name: selected.name.clone(),
        wit: wit.to_vec(),
        imports: alloc::vec::Vec::new(),
        exports: alloc::vec::Vec::new(),
        resources,
        asynchronous: false,
        checked_u64,
        worker_program_kind: None,
        worker_program_effect: None,
        worker_transition_kind: None,
        limits,
    };
    let context = Context {
        package: &package,
        selected,
    };
    at = 0;
    let mut failure = None;
    while let Some(item) = selected.items.get(at) {
        if let Err(problem) = context.item(item, &mut world) {
            failure = Some(problem);
            break;
        }
        at = at.saturating_add(1);
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    if world.imports.len() > crate::component::MAX_OPERATIONS
        || world.exports.len() > crate::component::MAX_OPERATIONS
    {
        return Err(crate::component::exhausted());
    }
    let is_unsigned = world
        .imports
        .iter()
        .chain(world.exports.iter())
        .any(|operation| {
            operation
                .parameters
                .iter()
                .chain(operation.results.iter())
                .any(|ty| *ty == crate::component::Type::CheckedU64)
        });
    if !checked_u64 && is_unsigned {
        return Err(crate::component::error(
            crate::component::Stage::Binding,
            crate::DiagnosticKind::Invalid,
            "WIT u64 requires an explicitly selected checked I64 boundary",
        ));
    }
    if checked_u64 && is_unsigned && !world.imports.is_empty() {
        return Err(crate::component::error(
            crate::component::Stage::Binding,
            crate::DiagnosticKind::Unsupported,
            "checked u64 boundary requires an import-free world",
        ));
    }
    if checked_u64
        && world.exports.iter().any(|operation| {
            let is_unsigned = operation
                .parameters
                .iter()
                .chain(operation.results.iter())
                .any(|ty| *ty == crate::component::Type::CheckedU64);
            is_unsigned
                && (world.asynchronous
                    || operation.asynchronous
                    || operation.parameters.len() != 1
                    || operation.results.len() != 1
                    || operation
                        .parameters
                        .iter()
                        .chain(operation.results.iter())
                        .any(|ty| *ty != crate::component::Type::CheckedU64))
        })
    {
        return Err(crate::component::error(
            crate::component::Stage::Binding,
            crate::DiagnosticKind::Unsupported,
            "checked u64 adapter supports only synchronous scalar operations",
        ));
    }
    attempt!(crate::component::bindings::make(&world));
    Ok(world)
}

fn qualified(package: &super::Package, name: &str) -> alloc::string::String {
    let capacity_bytes = package
        .namespace
        .len()
        .saturating_add(package.name.len())
        .saturating_add(name.len())
        .saturating_add(package.version.len())
        .saturating_add(3);
    let mut result = alloc::string::String::with_capacity(capacity_bytes);
    result.push_str(&package.namespace);
    result.push(':');
    result.push_str(&package.name);
    result.push('/');
    result.push_str(name);
    result.push('@');
    result.push_str(&package.version);
    result
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; this private concatenation primitive deliberately preserves prefix/separator/suffix order, with literal separators and named identity fields at every call; wrapper types would not validate the already checked WIT grammar."
)]
fn joined(prefix: &str, separator: &str, suffix: &str) -> alloc::string::String {
    let capacity_bytes = prefix
        .len()
        .saturating_add(separator.len())
        .saturating_add(suffix.len());
    let mut result = alloc::string::String::with_capacity(capacity_bytes);
    result.push_str(prefix);
    result.push_str(separator);
    result.push_str(suffix);
    result
}

fn method_name(resource: &str, function: &super::Function) -> alloc::string::String {
    let capacity_bytes = resource
        .len()
        .saturating_add(function.name.len())
        .saturating_add(9);
    let mut result = alloc::string::String::with_capacity(capacity_bytes);
    result.push_str("[method]");
    result.push_str(resource);
    result.push('.');
    result.push_str(&function.name);
    result
}

impl Context<'_> {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; item resolution searches runtime names and installs fallibly checked, allocating operation descriptors in the compiler-owned world."
    )]
    fn item(
        self,
        item: &super::Item,
        world: &mut crate::component::World,
    ) -> Result<(), crate::component::Error> {
        match item {
            super::Item::Use(usage) => {
                attempt!(types::resource(
                    &world.resources,
                    Some(&usage.interface),
                    &usage.name,
                    self.selected
                ));
                Ok(())
            }
            super::Item::Function(is_imported, function) => {
                self.install(None, function, *is_imported, world)
            }
            super::Item::Interface(is_imported, name) => {
                let position = self
                    .package
                    .interfaces
                    .iter()
                    .position(|interface| &interface.name == name);
                let index = match position {
                    Some(index) => index,
                    None => return Err(crate::component::invalid("unknown local WIT interface")),
                };
                self.install_interface(index, *is_imported, world)
            }
        }
    }

    fn install(
        self,
        interface: Option<&str>,
        function: &super::Function,
        is_imported: bool,
        world: &mut crate::component::World,
    ) -> Result<(), crate::component::Error> {
        let (parameters, results) =
            attempt!(self.signature(interface, function, &world.resources, is_imported));
        world.asynchronous |=
            function.asynchronous || contains_live(&parameters) || contains_live(&results);
        let name = match &function.method {
            Some(resource) => method_name(resource, function),
            None => function.name.clone(),
        };
        let module = match interface {
            Some(interface) => qualified(self.package, interface),
            None => alloc::string::String::from("$root"),
        };
        let identity_base = match interface {
            Some(interface) => qualified(self.package, interface),
            None => qualified(self.package, &self.selected.name),
        };
        let word = match &function.method {
            Some(resource) => joined(resource, ".", &function.name),
            None => match interface {
                Some(interface) => joined(interface, ".", &function.name),
                None => function.name.clone(),
            },
        };
        let export_name = match interface {
            Some(_) => joined(&module, "#", &name),
            None => name.clone(),
        };
        let count = world.imports.len();
        let definition = if is_imported {
            Some(noble_kernel::contracts::Definition(attempt!(number(
                count.saturating_add(BOOTSTRAP_DEFINITIONS)
            ))))
        } else {
            None
        };
        let effect = if is_imported {
            Some(noble_kernel::types::EffId(attempt!(number(
                count.saturating_add(BOOTSTRAP_EFFECTS)
            ))))
        } else {
            None
        };
        let operation = crate::component::Operation {
            identity: joined(&identity_base, "#", &name),
            word,
            core_module: module,
            core_name: name,
            export_name,
            parameters,
            results,
            asynchronous: function.asynchronous,
            effect,
            definition,
        };
        let target = if is_imported {
            &mut world.imports
        } else {
            &mut world.exports
        };
        if target.len() >= crate::component::MAX_OPERATIONS {
            return Err(crate::component::exhausted());
        }
        if target
            .iter()
            .any(|old| old.word == operation.word || old.export_name == operation.export_name)
        {
            return Err(crate::component::invalid(
                "duplicate or ambiguous generated WIT word",
            ));
        }
        target.push(operation);
        Ok(())
    }
}

const fn contains_live(types: &[crate::component::Type]) -> bool {
    let mut at = 0;
    while at < types.len() {
        if matches!(
            types[at],
            crate::component::Type::StreamU8
                | crate::component::Type::FutureS64
                | crate::component::Type::FutureResultS64String
        ) {
            return true;
        }
        at = at.saturating_add(1);
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; checked conversion uses non-const TryFrom and constructs an owned limit diagnostic if an identity cannot be represented."
)]
fn number(value: usize) -> Result<u32, crate::component::Error> {
    match u32::try_from(value) {
        Ok(value) => Ok(value),
        Err(_) => Err(crate::component::exhausted()),
    }
}
