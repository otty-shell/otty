/// What the tabs read from outside while they reduce: the shell new
/// terminal tabs are named after. The binary builds it before every intent,
/// so a tab opened right after the shell changed takes the new name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabsCtx<'a> {
    /// Name of the user's shell, such as `zsh`, given to new terminal tabs.
    pub shell_name: &'a str,
}
