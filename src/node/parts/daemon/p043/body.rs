
fn live_ticket_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
) -> Vec<String> {
    live_ticket_import_diagnostics(
        &ControlLiveTicketImportInput {
            state_root: Path::new("."),
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: input.expected_node,
            expected_topic: input.expected_topic,
            expected_endpoint: input.expected_endpoint,
            expected_peer: input.expected_peer,
            as_of_sequence: input.as_of_sequence,
        },
        ticket,
        Some(admission),
    )
}
