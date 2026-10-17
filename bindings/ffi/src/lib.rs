//! C ABI of the SUNAT SDK: the commands of the `sunat-sdk` CLI (docs/cli.md) in process, through
//! one function that takes a JSON request and gives a JSON response (docs/c-abi.md).
//!
//! Every function uses the platform's system calling convention (`extern "system"`): `__stdcall`
//! on 32-bit Windows, the one VB6, Visual FoxPro and PowerBuilder declare by default, and the C
//! convention everywhere else. The exported names carry no decoration.

mod request;

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

/// The response is the command's output (the CLI's stdout).
pub const SUNAT_SDK_OK: i32 = 0;
/// The response is `{"error": "<message>"}` (the CLI's stderr with exit code 1).
pub const SUNAT_SDK_ERROR: i32 = 1;
/// `response` was null: nothing ran and nothing was allocated.
pub const SUNAT_SDK_NO_RESPONSE_SLOT: i32 = 2;
/// `sunat_sdk_response_copy`: the buffer is smaller than the response; nothing was written.
pub const SUNAT_SDK_BUFFER_TOO_SMALL: i32 = 3;

/// Bumped only when a function, a status or the request shape changes incompatibly.
pub const SUNAT_SDK_ABI_VERSION: u32 = 1;

static VERSION: &CStr = match CStr::from_bytes_with_nul(concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes()) {
    Ok(version) => version,
    Err(_) => panic!("the package version has a NUL byte"),
};

/// Runs one command of the CLI. `request` is NUL-terminated UTF-8 JSON:
/// `{"command": "sign", "flags": {"cert": "...", "key": "..."}, "input": {...}}`.
///
/// Returns `SUNAT_SDK_OK` or `SUNAT_SDK_ERROR` and stores in `*response` a NUL-terminated UTF-8
/// JSON string owned by the library, which the caller releases with `sunat_sdk_free` exactly
/// once. Returns `SUNAT_SDK_NO_RESPONSE_SLOT` when `response` is null.
///
/// The call blocks for as long as the command runs (network commands wait up to their
/// `timeout` flag). It keeps no state between calls and may run on any thread, concurrently.
///
/// # Safety
/// `request` is null or points to a NUL-terminated string; `response` is null or points to
/// writable storage for one pointer.
#[no_mangle]
pub unsafe extern "system" fn sunat_sdk_call(request: *const c_char, response: *mut *mut c_char) -> i32 {
    if response.is_null() {
        return SUNAT_SDK_NO_RESPONSE_SLOT;
    }
    let (status, json) = catch_unwind(AssertUnwindSafe(|| {
        if request.is_null() {
            return (SUNAT_SDK_ERROR, request::error("request is null"));
        }
        match CStr::from_ptr(request).to_str() {
            Ok(text) => request::handle(text),
            Err(_) => (SUNAT_SDK_ERROR, request::error("request is not UTF-8")),
        }
    }))
    .unwrap_or_else(|_| (SUNAT_SDK_ERROR, request::error("internal error")));
    // JSON escapes every control character, so the text has no NUL byte.
    let json = CString::new(json).unwrap_or_else(|_| c"{\"error\":\"internal error\"}".to_owned());
    *response = json.into_raw();
    status
}

/// Releases a response of `sunat_sdk_call`. Null is ignored.
///
/// # Safety
/// `response` is null or a pointer stored by `sunat_sdk_call` and not yet released.
#[no_mangle]
pub unsafe extern "system" fn sunat_sdk_free(response: *mut c_char) {
    if !response.is_null() {
        drop(CString::from_raw(response));
    }
}

/// Length in bytes of a response, without the terminating NUL; 0 for null. For callers that
/// cannot read through a pointer (VB6, Visual FoxPro, PowerBuilder): size a buffer with this,
/// then fill it with `sunat_sdk_response_copy`.
///
/// # Safety
/// `response` is null or a live pointer stored by `sunat_sdk_call`.
#[no_mangle]
pub unsafe extern "system" fn sunat_sdk_response_len(response: *const c_char) -> usize {
    if response.is_null() {
        return 0;
    }
    CStr::from_ptr(response).to_bytes().len()
}

/// Copies a response's bytes into `buffer`, followed by a NUL when there is room for it.
/// Returns `SUNAT_SDK_OK`, or `SUNAT_SDK_BUFFER_TOO_SMALL` (nothing written) when `capacity` is
/// below `sunat_sdk_response_len`, or `SUNAT_SDK_ERROR` when a pointer is null. The response stays
/// owned by the library: release it with `sunat_sdk_free` after copying.
///
/// # Safety
/// `response` is a live pointer stored by `sunat_sdk_call`; `buffer` points to `capacity`
/// writable bytes.
#[no_mangle]
pub unsafe extern "system" fn sunat_sdk_response_copy(response: *const c_char, buffer: *mut c_char, capacity: usize) -> i32 {
    if response.is_null() || buffer.is_null() {
        return SUNAT_SDK_ERROR;
    }
    let bytes = CStr::from_ptr(response).to_bytes();
    if capacity < bytes.len() {
        return SUNAT_SDK_BUFFER_TOO_SMALL;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, bytes.len());
    if capacity > bytes.len() {
        *buffer.add(bytes.len()) = 0;
    }
    SUNAT_SDK_OK
}

/// The SDK version (`Cargo.toml`), a static NUL-terminated string: never released.
#[no_mangle]
pub extern "system" fn sunat_sdk_version() -> *const c_char {
    VERSION.as_ptr()
}

/// `SUNAT_SDK_ABI_VERSION` of the loaded library, so a wrapper can refuse one it was not built for.
#[no_mangle]
pub extern "system" fn sunat_sdk_abi_version() -> u32 {
    SUNAT_SDK_ABI_VERSION
}
