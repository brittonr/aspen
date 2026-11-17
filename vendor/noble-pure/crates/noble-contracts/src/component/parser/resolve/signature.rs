impl super::Context<'_> {
    pub(super) fn signature(
        self,
        interface: Option<&str>,
        function: &super::super::Function,
        resources: &[crate::component::Resource],
        is_imported: bool,
    ) -> Result<
        (
            alloc::vec::Vec<crate::component::Type>,
            alloc::vec::Vec<crate::component::Type>,
        ),
        crate::component::Error,
    > {
        let mut parameters = alloc::vec::Vec::with_capacity(function.parameters.len());
        let mut results = alloc::vec::Vec::with_capacity(function.results.len());
        let mut at = 0usize;
        let mut failure = None;
        while at < function.parameters.len() {
            let value = match super::types::ty(
                &function.parameters[at],
                interface,
                self.selected,
                resources,
            ) {
                Ok(value) => value,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            };
            if function.asynchronous && matches!(value, crate::component::Type::Borrow(_)) {
                failure = Some(crate::component::unsupported(
                    "an async WIT operation cannot retain a borrow",
                ));
                break;
            }
            if !is_imported && matches!(value, crate::component::Type::Borrow(_)) {
                failure = Some(crate::component::unsupported(
                    "borrowed exports are outside the synchronous subset",
                ));
                break;
            }
            parameters.push(value);
            at = at.saturating_add(1);
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        let flat_count = parameters.iter().fold(0usize, |count, ty| {
            count.saturating_add(match ty {
                crate::component::Type::String | crate::component::Type::Bytes => 2,
                crate::component::Type::ResultS64String
                | crate::component::Type::ResultBytesString => 3,
                crate::component::Type::Boolean
                | crate::component::Type::S64
                | crate::component::Type::CheckedU64
                | crate::component::Type::StreamU8
                | crate::component::Type::FutureS64
                | crate::component::Type::FutureResultS64String
                | crate::component::Type::Own(_)
                | crate::component::Type::Borrow(_) => 1,
            })
        });
        if flat_count > super::MAX_FLAT_PARAMETERS {
            return Err(crate::component::unsupported(
                "indirect parameter tuples are outside the bounded canonical adapter",
            ));
        }
        at = 0;
        while at < function.results.len() {
            let value = match super::types::ty(
                &function.results[at],
                interface,
                self.selected,
                resources,
            ) {
                Ok(value) => value,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            };
            if matches!(value, crate::component::Type::Borrow(_)) {
                failure = Some(crate::component::unsupported(
                    "a WIT borrow cannot escape a call scope",
                ));
                break;
            }
            results.push(value);
            at = at.saturating_add(1);
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok((parameters, results)),
        }
    }
}
