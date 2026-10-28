#![no_std]
mod app;
struct Component;
impl app::Guest for Component { fn run() -> u32 { 42 } }
app::export!(Component with_types_in app);
#[unsafe(export_name = "wizer.initialize")]
pub extern "C" fn wizer_initialize() {}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { loop {} }
