//! The time.

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::clock::now_ms()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::now_ms()
    }
}
