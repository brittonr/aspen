//! Simple echo worker for Hyperlight VM job execution.
//!
//! This worker demonstrates the basic guest binary pattern using official
//! Hyperlight guest libraries. It echoes back its input with a prefix.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hyperlight_common::flatbuffer_wrappers::function_call::FunctionCall;
use hyperlight_common::flatbuffer_wrappers::function_types::ParameterType;
use hyperlight_common::flatbuffer_wrappers::function_types::ParameterValue;
use hyperlight_common::flatbuffer_wrappers::function_types::ReturnType;
use hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode;
use hyperlight_guest::error::HyperlightGuestError;
use hyperlight_guest::error::Result;
use hyperlight_guest_bin::guest_function::definition::GuestFunctionDefinition;
use hyperlight_guest_bin::guest_function::register::register_function;

/// The execute function called by the host.
///
/// Takes VecBytes input and returns VecBytes output with "Echo: " prefix.
/// Function signature matches GuestFunc: fn(&FunctionCall) -> Result<Vec<u8>>
fn execute_impl(function_call: &FunctionCall) -> Result<Vec<u8>> {
    // Extract the input bytes from parameters
    let input_bytes = function_call
        .parameters
        .as_ref()
        .and_then(|params| params.first())
        .and_then(|p| {
            if let ParameterValue::VecBytes(bytes) = p {
                Some(bytes.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Convert to string for processing
    let input_str = match core::str::from_utf8(&input_bytes) {
        Ok(s) => s,
        Err(_) => "[invalid UTF-8]",
    };

    // Echo back with prefix
    let response = format!("Echo: {}", input_str);

    // Return serialized result as flatbuffer
    let bytes = hyperlight_common::flatbuffer_wrappers::util::get_flatbuffer_result(response.as_bytes());
    Ok(bytes)
}

/// The guest dispatch function - fallback for unregistered functions.
///
/// This function is called by Hyperlight when a function is not found
/// in the registered function map.
#[unsafe(no_mangle)]
pub extern "Rust" fn guest_dispatch_function(function_call: FunctionCall) -> Result<Vec<u8>> {
    Err(HyperlightGuestError::new(
        ErrorCode::GuestFunctionNotFound,
        format!("Unknown function: {}", function_call.function_name),
    ))
}

/// Entry point for the Hyperlight VM.
///
/// Registers guest functions that can be called by the host.
#[unsafe(no_mangle)]
pub extern "C" fn hyperlight_main() {
    // Register the execute function with Hyperlight
    // Takes VecBytes (raw bytes) and returns VecBytes
    let execute_def = GuestFunctionDefinition::new(
        String::from("execute"),
        Vec::from([ParameterType::VecBytes]),
        ReturnType::VecBytes,
        execute_impl as usize,
    );
    register_function(execute_def);
}
