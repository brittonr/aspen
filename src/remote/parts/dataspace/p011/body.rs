// The service-ready subscription the two-peer harness consumer uses.
//
// The consumer subscribes to the service-ready record shape, not to one exact
// value: a producer that adds a field keeps matching the subscription.

pub fn service_ready_subscription_pattern() -> Result<RuntimeValue> {
    RuntimeValue::new(
        crate::runtime::RuntimePattern::record(
            "service-ready",
            vec![crate::runtime::RecordPatternEntry {
                position: 0,
                pattern: Box::new(crate::runtime::RuntimePattern::wildcard("service")),
            }],
        )
        .to_value(),
    )
}
