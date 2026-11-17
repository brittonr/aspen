pub(super) fn ty(
    ty: &super::super::RawType,
    interface: Option<&str>,
    selected: &super::super::RawWorld,
    resources: &[crate::component::Resource],
) -> Result<crate::component::Type, crate::component::Error> {
    match ty {
        super::super::RawType::Bool => Ok(crate::component::Type::Boolean),
        super::super::RawType::S64 => Ok(crate::component::Type::S64),
        super::super::RawType::U64 => Ok(crate::component::Type::CheckedU64),
        super::super::RawType::String => Ok(crate::component::Type::String),
        super::super::RawType::Bytes => Ok(crate::component::Type::Bytes),
        super::super::RawType::ResultS64String => Ok(crate::component::Type::ResultS64String),
        super::super::RawType::ResultBytesString => Ok(crate::component::Type::ResultBytesString),
        super::super::RawType::StreamU8 => Ok(crate::component::Type::StreamU8),
        super::super::RawType::FutureS64 => Ok(crate::component::Type::FutureS64),
        super::super::RawType::FutureResultS64String => {
            Ok(crate::component::Type::FutureResultS64String)
        }
        super::super::RawType::Own(name) => Ok(crate::component::Type::Own(attempt!(resource(
            resources, interface, name, selected
        )))),
        super::super::RawType::Borrow(name) => Ok(crate::component::Type::Borrow(attempt!(
            resource(resources, interface, name, selected)
        ))),
    }
}

pub(super) fn resource(
    resources: &[crate::component::Resource],
    interface: Option<&str>,
    name: &str,
    selected: &super::super::RawWorld,
) -> Result<noble_kernel::types::ResourceKind, crate::component::Error> {
    match interface {
        Some(interface) => scoped_resource(resources, interface, name),
        None => {
            let index = match attempt!(use_index(selected, name)) {
                Some(index) => index,
                None => {
                    return Err(crate::component::unsupported(
                        "unknown resource or unsupported WIT type",
                    ))
                }
            };
            let usage = match selected.items.get(index) {
                Some(super::super::Item::Use(usage)) => usage,
                Some(_) | None => {
                    return Err(crate::component::unsupported(
                        "unknown resource or unsupported WIT type",
                    ))
                }
            };
            scoped_resource(resources, &usage.interface, name)
        }
    }
}

#[expect(
    tigerstyle::ambiguous_params,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this private resolver's interface and member name are separately named at both callsites and matched against their corresponding fields; iterator search, string equality and owned diagnostics are non-const."
)]
fn scoped_resource(
    resources: &[crate::component::Resource],
    interface: &str,
    name: &str,
) -> Result<noble_kernel::types::ResourceKind, crate::component::Error> {
    let position = resources
        .iter()
        .position(|resource| resource.name == name && resource.interface == interface);
    let index = match position {
        Some(index) => index,
        None => {
            return Err(crate::component::unsupported(
                "unknown resource or unsupported WIT type",
            ))
        }
    };
    match resources.get(index) {
        Some(resource) => Ok(resource.kind),
        None => Err(crate::component::unsupported(
            "unknown resource or unsupported WIT type",
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the finite immutable use scan returns an explicit ambiguity diagnostic rather than asserting namespace uniqueness on parsed input."
)]
fn use_index(
    selected: &super::super::RawWorld,
    name: &str,
) -> Result<Option<usize>, crate::component::Error> {
    let mut scope = None;
    let mut at = 0usize;
    let mut failure = None;
    while let Some(item) = selected.items.get(at) {
        if let super::super::Item::Use(usage) = item {
            if usage.name == name {
                if scope.is_some() {
                    failure = Some(crate::component::invalid("ambiguous WIT resource use"));
                    break;
                }
                scope = Some(at);
            }
        }
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(scope),
    }
}
