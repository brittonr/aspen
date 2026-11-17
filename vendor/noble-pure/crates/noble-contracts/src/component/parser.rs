#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; WIT parsing and resolution mutate only a fresh token cursor, syntax vectors and selected-world builder under fixed byte/token/declaration/operation bounds and the supplied work budget; borrowed WIT and committed sessions remain unchanged."
)]

mod cursor;
mod function;
mod lexer;
mod resolve;
mod world;

const MAX_DECLARATIONS: usize = 64;
const MAX_ITEMS: usize = 128;
const MAX_NAME_BYTES: usize = 128;
const VERSION_PARTS: u32 = 3;
const MAX_VERSION_DIGITS: usize = 10;
const MAX_VERSION_BYTES: usize = 32;

// These are exactly the parsed boundary profiles, not extensible fallbacks.
#[octet::sealed_enum]
#[derive(Clone)]
enum RawType {
    Bool,
    S64,
    U64,
    String,
    Bytes,
    ResultS64String,
    ResultBytesString,
    StreamU8,
    FutureS64,
    FutureResultS64String,
    Own(alloc::string::String),
    Borrow(alloc::string::String),
}
#[derive(Clone)]
struct Function {
    name: alloc::string::String,
    method: Option<alloc::string::String>,
    parameters: alloc::vec::Vec<RawType>,
    results: alloc::vec::Vec<RawType>,
    asynchronous: bool,
}
struct Interface {
    name: alloc::string::String,
    resources: alloc::vec::Vec<alloc::string::String>,
    functions: alloc::vec::Vec<Function>,
}
struct Use {
    interface: alloc::string::String,
    name: alloc::string::String,
}
#[octet::sealed_enum]
enum Item {
    Interface(bool, alloc::string::String),
    Function(bool, Function),
    Use(Use),
}
struct RawWorld {
    name: alloc::string::String,
    items: alloc::vec::Vec<Item>,
}
struct Package {
    namespace: alloc::string::String,
    name: alloc::string::String,
    version: alloc::string::String,
    interfaces: alloc::vec::Vec<Interface>,
    worlds: alloc::vec::Vec<RawWorld>,
}
struct Cursor<'a> {
    source: &'a str,
    tokens: alloc::vec::Vec<core::ops::Range<usize>>,
    at: usize,
    remaining: u32,
}

pub(super) fn parse(
    wit: &[u8],
    selected: &str,
    limits: crate::Limits,
    checked_u64: bool,
) -> Result<super::World, super::Error> {
    let mut parser = attempt!(lexer::cursor(wit, limits));
    let package = attempt!(parser.package());
    resolve::world(package, wit, selected, limits, checked_u64)
}

impl Cursor<'_> {
    fn package(&mut self) -> Result<Package, super::Error> {
        attempt!(self.take("package"));
        let namespace = attempt!(self.name());
        attempt!(self.take(":"));
        let name = attempt!(self.name());
        attempt!(self.take("@"));
        let version = attempt!(self.version());
        attempt!(self.take(";"));
        let mut package = Package {
            namespace,
            name,
            version,
            interfaces: alloc::vec::Vec::new(),
            worlds: alloc::vec::Vec::new(),
        };
        let mut failure = None;
        while self.at < self.tokens.len() {
            if let Err(problem) = self.declaration(&mut package) {
                failure = Some(problem);
                break;
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(package),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; declaration dispatch invokes allocating interface/world parsers, appends owned declarations and produces runtime syntax diagnostics."
    )]
    fn declaration(&mut self, package: &mut Package) -> Result<(), super::Error> {
        if package
            .interfaces
            .len()
            .saturating_add(package.worlds.len())
            >= MAX_DECLARATIONS
        {
            return Err(super::exhausted());
        }
        match attempt!(self.next()) {
            "interface" => package.interfaces.push(attempt!(self.interface())),
            "world" => package.worlds.push(attempt!(self.world())),
            _ => {
                return Err(super::unsupported(
                    "only local versioned WIT interfaces and worlds are supported",
                ))
            }
        }
        Ok(())
    }

    fn interface(&mut self) -> Result<Interface, super::Error> {
        let name = attempt!(self.name());
        attempt!(self.take("{"));
        let mut interface = Interface {
            name,
            resources: alloc::vec::Vec::new(),
            functions: alloc::vec::Vec::new(),
        };
        let mut failure = None;
        while !self.peek("}") {
            if let Err(problem) = self.interface_item(&mut interface) {
                failure = Some(problem);
                break;
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        attempt!(self.take("}"));
        Ok(interface)
    }

    fn interface_item(&mut self, interface: &mut Interface) -> Result<(), super::Error> {
        if interface.functions.len() >= super::MAX_OPERATIONS {
            return Err(super::exhausted());
        }
        if self.peek("resource") {
            self.resource(interface)
        } else {
            let name = attempt!(self.name());
            let function = attempt!(self.function(name, None));
            interface.functions.push(function);
            Ok(())
        }
    }

    fn resource(&mut self, interface: &mut Interface) -> Result<(), super::Error> {
        attempt!(self.take("resource"));
        let name = attempt!(self.name());
        if interface.resources.len() >= super::MAX_RESOURCES || interface.resources.contains(&name)
        {
            return Err(super::invalid("duplicate or excessive WIT resource"));
        }
        interface.resources.push(name.clone());
        if self.peek(";") {
            return self.take(";");
        }
        attempt!(self.take("{"));
        let mut failure = None;
        while !self.peek("}") {
            if let Err(problem) = self.method(&name, interface) {
                failure = Some(problem);
                break;
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        self.take("}")
    }

    fn method(&mut self, resource: &str, interface: &mut Interface) -> Result<(), super::Error> {
        if interface.functions.len() >= super::MAX_OPERATIONS {
            return Err(super::exhausted());
        }
        let method = attempt!(self.name());
        let function = attempt!(self.function(method, Some(alloc::string::String::from(resource))));
        interface.functions.push(function);
        Ok(())
    }
}
