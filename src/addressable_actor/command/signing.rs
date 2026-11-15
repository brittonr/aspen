use std::io::BufRead;
use std::io::Read;
use std::io::Write;

pub(super) struct Dialogue {
    bundle_ref: String,
}
impl Dialogue {
    pub(super) fn new(bundle_ref: String) -> Self {
        Self { bundle_ref }
    }
}

impl crate::addressable_actor::native::SigningPort for Dialogue {
    fn prepare(&mut self, request: &crate::node_service_hosting::OperationRequest) -> crate::error::Result<()> {
        let reference = crate::node_service_host::authority::request_ref(&self.bundle_ref, request)?;
        let text = crate::preserves_rail::to_text(&crate::node_service_host::command::request_value(request))?;
        if text.contains('\n') || text.len() > 65_536 {
            return Err(super::invalid("signing preview exceeds line bound"));
        }
        {
            let mut output = std::io::stdout().lock();
            writeln!(output, "SIGN {text}").map_err(crate::error::Failure::from)?;
            output.flush().map_err(crate::error::Failure::from)?;
        }
        let mut answer = String::new();
        std::io::stdin().lock().take(4098).read_line(&mut answer).map_err(crate::error::Failure::from)?;
        exact_ready(&answer, &reference)
    }
}

pub(super) fn exact_ready(answer: &str, reference: &str) -> crate::error::Result<()> {
    if answer != format!("READY {reference}\n") {
        return Err(super::invalid("independent signing handshake does not bind exact current request"));
    }
    // This handshake grants nothing. Current subsequently reads and verifies the rooted credential.
    Ok(())
}
