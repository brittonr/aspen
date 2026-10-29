//! Exact structural type bookkeeping for the product's root byte carrier.

pub(super) struct RootTypes {
    byte_lists: Vec<bool>,
}

impl RootTypes {
    pub(super) fn new() -> Self {
        Self {
            byte_lists: Vec::with_capacity(super::observation::MAX_OBSERVED_SURFACE_ENTRIES),
        }
    }

    pub(super) fn observe(
        &mut self,
        section: wasmparser::ComponentTypeSectionReader<'_>,
    ) -> super::super::model::Result<()> {
        for item in section {
            let is_byte_list = matches!(
                item.map_err(super::observation::parse_denial)?,
                wasmparser::ComponentType::Defined(wasmparser::ComponentDefinedType::List(
                    wasmparser::ComponentValType::Primitive(wasmparser::PrimitiveValType::U8)
                ))
            );
            self.push(is_byte_list)?;
        }
        Ok(())
    }

    pub(super) fn aliases(
        &mut self,
        section: wasmparser::ComponentAliasSectionReader<'_>,
    ) -> super::super::model::Result<()> {
        for alias in section {
            match alias.map_err(super::observation::parse_denial)? {
                wasmparser::ComponentAlias::Outer {
                    kind: wasmparser::ComponentOuterAliasKind::Type,
                    count: 0,
                    index,
                } => {
                    let is_byte_list = self.is_byte_list(index);
                    self.push(is_byte_list)?;
                }
                wasmparser::ComponentAlias::Outer {
                    kind: wasmparser::ComponentOuterAliasKind::Type,
                    ..
                }
                | wasmparser::ComponentAlias::InstanceExport {
                    kind: wasmparser::ComponentExternalKind::Type,
                    ..
                } => {
                    // Instance/outer authority never becomes an admitted structural alias.
                    self.push(false)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn import(
        &mut self,
        world: &str,
        import: &wasmparser::ComponentImport<'_>,
    ) -> super::super::model::Result<bool> {
        match import.ty {
            wasmparser::ComponentTypeRef::Func(_) | wasmparser::ComponentTypeRef::Instance(_) => Ok(true),
            wasmparser::ComponentTypeRef::Type(wasmparser::TypeBounds::Eq(index))
                if world == super::super::effect::HOST_WIT_WORLD
                    && import.name.0 == "preserves-payload"
                    && self.is_byte_list(index) =>
            {
                self.push(true)?;
                Ok(false)
            }
            wasmparser::ComponentTypeRef::Module(_)
            | wasmparser::ComponentTypeRef::Value(_)
            | wasmparser::ComponentTypeRef::Type(_)
            | wasmparser::ComponentTypeRef::Component(_) => {
                Err(super::super::model::Denial::new("component imports an unsupported type, resource or module"))
            }
        }
    }

    fn is_byte_list(&self, index: u32) -> bool {
        usize::try_from(index).ok().and_then(|index| self.byte_lists.get(index)).copied() == Some(true)
    }

    fn push(&mut self, is_byte_list: bool) -> super::super::model::Result<()> {
        if self.byte_lists.len() == super::observation::MAX_OBSERVED_SURFACE_ENTRIES {
            return Err(super::super::model::Denial::new("component root type bookkeeping exceeds its bound"));
        }
        self.byte_lists.push(is_byte_list);
        Ok(())
    }
}
