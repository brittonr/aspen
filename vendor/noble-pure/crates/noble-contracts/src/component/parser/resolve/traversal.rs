pub(super) fn is_selected(package: &super::super::Package, at: usize, selected: &str) -> bool {
    let world = &package.worlds[at];
    world.name == selected || super::qualified(package, &world.name) == selected
}

impl super::Context<'_> {
    pub(super) fn install_interface(
        self,
        interface_index: usize,
        is_imported: bool,
        world: &mut crate::component::World,
    ) -> Result<(), crate::component::Error> {
        let count = attempt!(self.interface_count(interface_index, is_imported));
        let mut at = 0usize;
        let mut failure = None;
        while at < count {
            if let Err(problem) = self.install_member(interface_index, at, is_imported, world) {
                failure = Some(problem);
                break;
            }
            at = at.saturating_add(1);
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; interface lookup uses non-const slice access and owned diagnostics to reject absent interfaces or unsupported resource exports."
    )]
    fn interface_count(
        self,
        index: usize,
        is_imported: bool,
    ) -> Result<usize, crate::component::Error> {
        let interface = match self.package.interfaces.get(index) {
            Some(interface) => interface,
            None => return Err(crate::component::invalid("unknown local WIT interface")),
        };
        if !is_imported && !interface.resources.is_empty() {
            return Err(crate::component::unsupported(
                "guest-defined resource exports require a destructor implementation",
            ));
        }
        Ok(interface.functions.len())
    }

    #[expect(
        tigerstyle::ambiguous_params,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; the private call names interface and member indices separately, checks each at its corresponding lookup, then invokes allocating, fallible operation installation."
    )]
    fn install_member(
        self,
        interface_index: usize,
        member_index: usize,
        is_imported: bool,
        world: &mut crate::component::World,
    ) -> Result<(), crate::component::Error> {
        let interface = match self.package.interfaces.get(interface_index) {
            Some(interface) => interface,
            None => return Err(crate::component::invalid("unknown local WIT interface")),
        };
        let function = match interface.functions.get(member_index) {
            Some(function) => function,
            None => return Err(crate::component::invalid("invalid WIT function index")),
        };
        self.install(Some(&interface.name), function, is_imported, world)
    }
}
