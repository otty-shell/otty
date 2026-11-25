//! The import allowlist: a plugin may import only the WASI parts an empty
//! context gives (`wasi:cli`, `wasi:io`) and OTTY's own interfaces.

/// The import name prefixes a plugin may use (spec "Engine limits").
const ALLOWED_IMPORTS: [&str; 3] = ["wasi:cli/", "wasi:io/", "otty:"];

/// The imports among `names` that are not allowed, such as
/// `wasi:filesystem/types@0.2.6`, in the order given.
pub(crate) fn refused_imports<'a>(
    names: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    names
        .filter(|name| {
            !ALLOWED_IMPORTS
                .iter()
                .any(|allowed| name.starts_with(allowed))
        })
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_io_and_otty_imports_are_allowed() {
        let refused = refused_imports(
            [
                "wasi:cli/stderr@0.2.6",
                "wasi:io/streams@0.2.6",
                "otty:host/files@0.1.0",
            ]
            .into_iter(),
        );

        assert!(refused.is_empty());
    }

    #[test]
    fn other_imports_are_refused_by_name() {
        let refused = refused_imports(
            [
                "wasi:cli/stderr@0.2.6",
                "wasi:filesystem/types@0.2.6",
                "wasi:sockets/tcp@0.2.6",
                "wasi:clocks/wall-clock@0.2.6",
                "wasi:clinic/x@0.1.0",
            ]
            .into_iter(),
        );

        assert_eq!(
            refused,
            [
                "wasi:filesystem/types@0.2.6",
                "wasi:sockets/tcp@0.2.6",
                "wasi:clocks/wall-clock@0.2.6",
                "wasi:clinic/x@0.1.0",
            ]
        );
    }
}
