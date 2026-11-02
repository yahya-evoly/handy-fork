//! FORK: safe wrapper over the Apple Translation bridge
//! (`swift/fork_translation.swift`, built by `build_fork.rs`).

/// Translates `text` from `source` to `target` on-device. `None` when the
/// platform can't (not Apple Silicon macOS 26+, language pair not installed,
/// or the translation failed).
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn translate(text: &str, source: &str, target: &str) -> Option<String> {
    use std::ffi::{c_char, CStr, CString};

    extern "C" {
        fn fork_translate_text(
            text: *const c_char,
            source: *const c_char,
            target: *const c_char,
        ) -> *mut c_char;
        fn fork_free_translation(pointer: *mut c_char);
    }

    let text = CString::new(text).ok()?;
    let source = CString::new(source).ok()?;
    let target = CString::new(target).ok()?;
    // SAFETY: all three are valid NUL-terminated strings that outlive the
    // call; a non-null result is a malloc'd C string we free exactly once.
    unsafe {
        let result = fork_translate_text(text.as_ptr(), source.as_ptr(), target.as_ptr());
        if result.is_null() {
            return None;
        }
        let translated = CStr::from_ptr(result).to_string_lossy().into_owned();
        fork_free_translation(result);
        let translated = translated.trim().to_string();
        (!translated.is_empty()).then_some(translated)
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub fn translate(_text: &str, _source: &str, _target: &str) -> Option<String> {
    None
}

// No unit test for `translate`: the Translation framework needs a free main
// run loop, which the test harness doesn't provide (it would just time out).
// The app has one; `fork_language::start_recording` logs whether the
// translator answered when it warms it up.
