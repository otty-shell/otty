//! The home folder and the variables the called part lists in `env`.

/// The variable `name` as text; `None` when it is not set or the part does
/// not list it. A variable that holds a key is read only through
/// [`keys::open`](crate::keys::open).
pub fn var(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::env::var(name)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::var(name)
    }
}

/// The user's home folder, what `~` stands for in the manifest.
pub fn home() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::env::home()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::home()
    }
}
