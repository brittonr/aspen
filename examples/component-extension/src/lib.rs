#![no_std]
extern crate alloc;

mod actor_effects;
mod allocator;
mod codec;
mod identity;
mod invocation;
mod packed;

use alloc::string::String;
use alloc::vec::Vec;

struct Extension;

impl actor_effects::Guest for Extension {
    fn invoke(input: Vec<u8>) -> Result<Vec<u8>, String> {
        let invocation = invocation::decode(&input)?;
        match invocation.callback {
            b"initialize" => {
                let mut initial = Vec::new();
                codec::start_record(&mut initial, "component-extension-state-v1");
                initial.push(0x84);
                Ok(codec::outcome(None, Some(&initial), None))
            }
            b"request" | b"message" => {
                let request = invocation.value.as_deref().ok_or_else(|| String::from("request callback requires a materialized payload"))?;
                // The actual host owns Session admission, authority and recordings.
                // A canonical denied response is still a successful carrier; it
                // remains intact in outputs/state instead of being upgraded to pass.
                let response = actor_effects::effect_request(request)?;
                codec::record::<4>(&response, "component-effect-response-v1")?;
                Ok(codec::outcome(Some(&response), Some(&response), None))
            }
            b"checkpoint" => {
                let state = invocation.value.as_deref().ok_or_else(|| String::from("checkpoint requires committed semantic state"))?;
                Ok(codec::outcome(None, None, Some(state)))
            }
            b"recover" => {
                let checkpoint = invocation.value.as_deref().ok_or_else(|| String::from("recover requires a materialized checkpoint"))?;
                Ok(codec::outcome(None, Some(checkpoint), None))
            }
            b"start" | b"drain" | b"shutdown" | b"health" => Ok(codec::outcome(None, None, None)),
            _ => Err(String::from("callback is unsupported by this installed guest")),
        }
    }
}

actor_effects::export!(Extension with_types_in actor_effects);

#[unsafe(export_name = "cabi_realloc")]
unsafe extern "C" fn cabi_realloc(old: *mut u8, old_bytes: usize, align: usize, new_bytes: usize) -> *mut u8 {
    if new_bytes == 0 {
        return align as *mut u8;
    }
    let layout = core::alloc::Layout::from_size_align(old_bytes.max(1), align).unwrap();
    let result = if old_bytes == 0 {
        unsafe { alloc::alloc::alloc(core::alloc::Layout::from_size_align(new_bytes, align).unwrap()) }
    } else {
        unsafe { alloc::alloc::realloc(old, layout, new_bytes) }
    };
    if result.is_null() {
        core::arch::wasm32::unreachable();
    }
    result
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
