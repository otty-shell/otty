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

/// How many minutes the user's clock is ahead of UTC at `at_ms`, in the
/// time zone of their system, such as 180 in Moscow; 0 when the system
/// tells none. The days and months of the user's own calendar start by
/// it.
///
/// ```no_run
/// use otty_plugin_sdk::clock;
///
/// let now_ms = clock::now_ms();
/// let offset_ms = i64::from(clock::local_offset_minutes(now_ms)) * 60_000;
/// let on_the_user_s_clock_ms = now_ms + offset_ms;
/// ```
pub fn local_offset_minutes(at_ms: i64) -> i32 {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::clock::local_offset_minutes(at_ms)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::local_offset_minutes(at_ms)
    }
}
