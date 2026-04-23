use aspen_hooks_ticket::AspenHookTicket;

fn main() {
    let ticket = AspenHookTicket::new("cluster", vec![]);
    let _ = ticket.with_expiry_hours(1);
}
