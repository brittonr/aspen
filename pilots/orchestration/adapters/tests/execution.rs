// SPDX-License-Identifier: AGPL-3.0-or-later
mod common;
use common::*;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;
#[tokio::test]
async fn actual_wasmtime_component_abi_returns_bytes() {
    let c = fixture();
    let r = request(content_id(&c).unwrap());
    let runtime = PureComponentRuntime::default();
    let result = runtime.run(r, c, vec![b"input".to_vec()]).await.unwrap();
    assert!(result.stopped);
    assert_eq!(result.output.unwrap(), b"ok");
}
#[tokio::test]
async fn cancellation_before_execution_is_latched() {
    let c = fixture();
    let r = request(content_id(&c).unwrap());
    let runtime = PureComponentRuntime::default();
    runtime.cancel(r.fence.assignment).unwrap();
    let result = runtime.run(r, c, vec![]).await.unwrap();
    assert!(result.stopped);
    assert_eq!(result.output, Err(FailureCode::Interrupted));
}
#[tokio::test]
async fn output_limit_rejects_after_stopped_execution() {
    let c = fixture();
    let mut r = request(content_id(&c).unwrap());
    r.max_output_bytes = 1;
    let result = PureComponentRuntime::default().run(r, c, vec![]).await.unwrap();
    assert!(result.stopped);
    assert_eq!(result.output, Err(FailureCode::OutputLimit));
}
#[tokio::test]
async fn no_host_imports_are_provided() {
    let text = String::from_utf8(fixture().bytes).unwrap().replacen(
        "(component",
        "(component (type $host-type (func)) (import \"ambient\" (func (type $host-type)))",
        1,
    );
    let c = content(ContentKind::Component, text.into_bytes()).unwrap();
    let r = request(content_id(&c).unwrap());
    let result = PureComponentRuntime::default().run(r, c, vec![]).await.unwrap();
    assert!(result.stopped);
    assert_eq!(result.output, Err(FailureCode::Instantiate));
}
#[tokio::test]
async fn memory_growth_above_grant_is_denied() {
    let text = String::from_utf8(fixture().bytes)
        .unwrap()
        .replace("(memory (export \"memory\") 1)", "(memory (export \"memory\") 2)");
    let c = content(ContentKind::Component, text.into_bytes()).unwrap();
    let mut r = request(content_id(&c).unwrap());
    r.resources.memory_bytes = 65536;
    let result = PureComponentRuntime::default().run(r, c, vec![]).await.unwrap();
    assert!(result.stopped);
    assert!(result.output.is_err());
}
#[tokio::test]
async fn infinite_component_is_bounded_by_fuel_or_deadline() {
    let text = String::from_utf8(fixture().bytes).unwrap().replace(
        "i32.const 16 i32.const 32 i32.store",
        "(loop $forever (br $forever)) i32.const 16 i32.const 32 i32.store",
    );
    let c = content(ContentKind::Component, text.into_bytes()).unwrap();
    let mut r = request(content_id(&c).unwrap());
    r.fuel = 1000;
    r.timeout_ms = 500;
    let result = PureComponentRuntime::default().run(r, c, vec![]).await.unwrap();
    assert!(result.stopped);
    assert!(result.output.is_err());
}
