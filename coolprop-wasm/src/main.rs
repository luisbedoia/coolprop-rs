//! Emscripten entry point: exposes the JSON API in `lib.rs` as `extern "C"`
//! functions. Every returned string is heap-allocated by Rust; JS must
//! release it with `coolprop_free_string`.

#[cfg(target_os = "emscripten")]
mod exports {
    use std::ffi::{CStr, CString, c_char};

    fn into_c(s: String) -> *mut c_char {
        // JSON never contains an interior NUL.
        CString::new(s).map_or(std::ptr::null_mut(), CString::into_raw)
    }

    /// Calls `f` with the request string, or reports a malformed pointer.
    ///
    /// # Safety
    /// `input` must be null or point to a NUL-terminated string.
    unsafe fn with_request(input: *const c_char, f: fn(&str) -> String) -> *mut c_char {
        if input.is_null() {
            return into_c(f("null"));
        }
        // SAFETY: non-null and NUL-terminated per this function's contract.
        let request = unsafe { CStr::from_ptr(input) }.to_string_lossy();
        into_c(f(&request))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn coolprop_version() -> *mut c_char {
        into_c(coolprop_wasm::version())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn coolprop_catalog() -> *mut c_char {
        into_c(coolprop_wasm::catalog())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn coolprop_schema() -> *mut c_char {
        into_c(coolprop_wasm::schema())
    }

    /// # Safety
    /// `input` must be null or a NUL-terminated UTF-8 string.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn coolprop_fluid(input: *const c_char) -> *mut c_char {
        // SAFETY: forwarded caller contract.
        unsafe { with_request(input, coolprop_wasm::fluid) }
    }

    /// # Safety
    /// `input` must be null or a NUL-terminated UTF-8 string.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn coolprop_state(input: *const c_char) -> *mut c_char {
        // SAFETY: forwarded caller contract.
        unsafe { with_request(input, coolprop_wasm::state) }
    }

    /// # Safety
    /// `input` must be null or a NUL-terminated UTF-8 string.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn coolprop_states(input: *const c_char) -> *mut c_char {
        // SAFETY: forwarded caller contract.
        unsafe { with_request(input, coolprop_wasm::states) }
    }

    /// # Safety
    /// `ptr` must be null or a string returned by one of the functions above,
    /// not yet freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn coolprop_free_string(ptr: *mut c_char) {
        if !ptr.is_null() {
            // SAFETY: allocated by `CString::into_raw` and freed only once.
            drop(unsafe { CString::from_raw(ptr) });
        }
    }

    /// Referenced from `main` so the linker keeps the exports.
    pub(crate) const ALL: [*const (); 7] = [
        coolprop_version as *const (),
        coolprop_catalog as *const (),
        coolprop_schema as *const (),
        coolprop_fluid as *const (),
        coolprop_state as *const (),
        coolprop_states as *const (),
        coolprop_free_string as *const (),
    ];
}

fn main() {
    #[cfg(target_os = "emscripten")]
    std::hint::black_box(exports::ALL);
}
