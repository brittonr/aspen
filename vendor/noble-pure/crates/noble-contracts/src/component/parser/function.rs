impl super::Cursor<'_> {
    pub(super) fn function(
        &mut self,
        name: alloc::string::String,
        method: Option<alloc::string::String>,
    ) -> Result<super::Function, crate::component::Error> {
        attempt!(self.take(":"));
        let is_async = self.peek("async");
        if is_async {
            attempt!(self.take("async"));
        }
        attempt!(self.take("func"));
        attempt!(self.take("("));
        let mut parameters = alloc::vec::Vec::new();
        let mut names = alloc::vec::Vec::with_capacity(crate::component::MAX_PARAMETERS);
        if let Some(resource) = &method {
            parameters.push(super::RawType::Borrow(resource.clone()));
        }
        let mut failure = None;
        while parameters.len() < crate::component::MAX_PARAMETERS && !self.peek(")") {
            match self.parameter(&names) {
                Ok((name, ty)) => {
                    names.push(name);
                    parameters.push(ty);
                }
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        if !self.peek(")") {
            return Err(crate::component::exhausted());
        }
        attempt!(self.take(")"));
        let mut results = alloc::vec::Vec::new();
        if self.peek("->") {
            attempt!(self.take("->"));
            results.push(attempt!(self.ty()));
        }
        attempt!(self.take(";"));
        Ok(super::Function {
            name,
            method,
            parameters,
            results,
            asynchronous: is_async,
        })
    }

    fn parameter(
        &mut self,
        names: &[alloc::string::String],
    ) -> Result<(alloc::string::String, super::RawType), crate::component::Error> {
        let name = attempt!(self.name());
        if names.contains(&name) {
            return Err(crate::component::invalid("duplicate WIT parameter"));
        }
        attempt!(self.take(":"));
        let ty = attempt!(self.ty());
        if !self.peek(")") {
            attempt!(self.take(","));
        }
        Ok((name, ty))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; type parsing consumes metered tokens, allocates owned resource names and returns owned diagnostics for unsupported adapters."
    )]
    fn ty(&mut self) -> Result<super::RawType, crate::component::Error> {
        let name = attempt!(self.next());
        match name {
            "bool" => Ok(super::RawType::Bool),
            "s64" => Ok(super::RawType::S64),
            "u64" => Ok(super::RawType::U64),
            "string" => Ok(super::RawType::String),
            "list" | "stream" => {
                attempt!(self.take("<"));
                attempt!(self.take("u8"));
                attempt!(self.take(">"));
                if name == "list" {
                    Ok(super::RawType::Bytes)
                } else {
                    Ok(super::RawType::StreamU8)
                }
            }
            "result" => self.result_type(),
            "future" => self.future(),
            "own" | "borrow" => {
                attempt!(self.take("<"));
                let resource = attempt!(self.name());
                attempt!(self.take(">"));
                if name == "own" {
                    Ok(super::RawType::Own(resource))
                } else {
                    Ok(super::RawType::Borrow(resource))
                }
            }
            "u8" | "s8" | "u16" | "s16" | "u32" | "s32" | "f32" | "f64" | "char" | "tuple"
            | "option" => Err(crate::component::unsupported(
                "WIT type has no exact supported bounded adapter",
            )),
            _ => {
                self.at = self.at.saturating_sub(1);
                Ok(super::RawType::Own(attempt!(self.name())))
            }
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; future parsing consumes metered tokens through non-const cursor methods and constructs owned diagnostics on unsupported payloads."
    )]
    fn future(&mut self) -> Result<super::RawType, crate::component::Error> {
        attempt!(self.take("<"));
        let ty = match attempt!(self.next()) {
            "s64" => super::RawType::FutureS64,
            "result" => {
                if !matches!(
                    attempt!(self.result_type()),
                    super::RawType::ResultS64String
                ) {
                    return Err(crate::component::unsupported(
                        "only future<s64> and future<result<s64,string>> have bounded adapters",
                    ));
                }
                super::RawType::FutureResultS64String
            }
            _ => {
                return Err(crate::component::unsupported(
                    "only future<s64> and future<result<s64,string>> have bounded adapters",
                ))
            }
        };
        attempt!(self.take(">"));
        Ok(ty)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; result parsing consumes metered tokens through non-const cursor methods and constructs owned diagnostics on unsupported payloads."
    )]
    fn result_type(&mut self) -> Result<super::RawType, crate::component::Error> {
        attempt!(self.take("<"));
        let ty = match attempt!(self.next()) {
            "s64" => super::RawType::ResultS64String,
            "list" => {
                attempt!(self.take("<"));
                attempt!(self.take("u8"));
                attempt!(self.take(">"));
                super::RawType::ResultBytesString
            }
            _ => {
                return Err(crate::component::unsupported(
                    "only result<s64,string> and result<list<u8>,string> have bounded adapters",
                ))
            }
        };
        attempt!(self.take(","));
        attempt!(self.take("string"));
        attempt!(self.take(">"));
        Ok(ty)
    }
}
