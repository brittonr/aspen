impl super::Cursor<'_> {
    pub(super) fn world(&mut self) -> Result<super::RawWorld, crate::component::Error> {
        let name = attempt!(self.name());
        attempt!(self.take("{"));
        let mut items = alloc::vec::Vec::new();
        let mut failure = None;
        while items.len() < super::MAX_ITEMS && !self.peek("}") {
            let result = if self.peek("use") {
                self.uses(&mut items)
            } else {
                match self.item() {
                    Ok(item) => {
                        items.push(item);
                        Ok(())
                    }
                    Err(problem) => Err(problem),
                }
            };
            if let Err(problem) = result {
                failure = Some(problem);
                break;
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        if !self.peek("}") {
            return Err(crate::component::exhausted());
        }
        attempt!(self.take("}"));
        Ok(super::RawWorld { name, items })
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; item dispatch consumes runtime tokens and invokes allocating name/function parsers with owned diagnostics."
    )]
    fn item(&mut self) -> Result<super::Item, crate::component::Error> {
        let is_imported = match attempt!(self.next()) {
            "import" => true,
            "export" => false,
            _ => return Err(crate::component::unsupported("unsupported WIT world item")),
        };
        let name = attempt!(self.name());
        if self.peek(";") {
            attempt!(self.take(";"));
            Ok(super::Item::Interface(is_imported, name))
        } else {
            Ok(super::Item::Function(
                is_imported,
                attempt!(self.function(name, None)),
            ))
        }
    }

    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; each parsed use appends one fresh syntax item after checking MAX_ITEMS; a slice cannot express this bounded growth of the parser-owned vector."
    )]
    fn uses(
        &mut self,
        items: &mut alloc::vec::Vec<super::Item>,
    ) -> Result<(), crate::component::Error> {
        attempt!(self.take("use"));
        let interface = attempt!(self.name());
        attempt!(self.take("."));
        attempt!(self.take("{"));
        let mut failure = None;
        while !self.peek("}") {
            match self.use_item(&interface, items.len()) {
                Ok(item) => items.push(item),
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        attempt!(self.take("}"));
        self.take(";")
    }

    fn use_item(
        &mut self,
        interface: &str,
        count: usize,
    ) -> Result<super::Item, crate::component::Error> {
        if count >= super::MAX_ITEMS {
            return Err(crate::component::exhausted());
        }
        let name = attempt!(self.name());
        let item = super::Item::Use(super::Use {
            interface: alloc::string::String::from(interface),
            name,
        });
        if !self.peek("}") {
            attempt!(self.take(","));
        }
        Ok(item)
    }
}
