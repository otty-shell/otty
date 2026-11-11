//! What other features know about a tab: the machine it runs on.

/// Machine a tab runs on. It is set once, when the tab opens: the quick
/// launch feature picks it for a launch, and the tabs feature groups its
/// Sessions list by it.
///
/// # Examples
///
/// ```
/// use otty_app_domain::tab::TabMachine;
///
/// let launch = TabMachine::Ssh(String::from("laptop"));
///
/// assert_ne!(launch, TabMachine::ThisComputer);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabMachine {
    /// This computer: shells, local launches and the launch editor.
    ThisComputer,
    /// The host an SSH launch connects to.
    Ssh(String),
}
