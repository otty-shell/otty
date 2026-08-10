use std::path::PathBuf;

/// Resolve the active terminal working directory from block metadata.
pub(crate) fn terminal_cwd_from_blocks(
    blocks: &[otty_ui_term::BlockSnapshot],
) -> Option<PathBuf> {
    blocks
        .iter()
        .rev()
        .find_map(|block| block.meta.cwd.as_deref())
        .map(PathBuf::from)
}
