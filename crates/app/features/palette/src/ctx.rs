use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
use otty_ui_kit::components::target_dot::TargetKind;

/// What the palette reads from outside while it reduces: what it lists and
/// which commands can run now. The binary builds it from the other features
/// before every intent, so the palette lists what is true when it opens and
/// on every query change, and checks a picked command against the moment it
/// is picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCtx<'a> {
    /// Open tabs, grouped by machine in the order the Sessions panel shows
    /// them.
    pub sessions: Vec<PaletteSession<'a>>,
    /// Every launch of the launch tree, in tree order.
    pub launches: Vec<PaletteLaunch<'a>>,
    /// Terminal and the pinned modes, in switcher order: the mode at
    /// position N has mode shortcut N.
    pub pinned_modes: Vec<Mode>,
    /// What decides which app commands can run now; only those are listed
    /// and run (`AppCommand::is_available`).
    pub commands: CommandCtx,
}

/// One open tab the palette can switch to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSession<'a> {
    /// Id of the tab.
    pub tab_id: u64,
    /// Title of the tab, the entry's title.
    pub title: &'a str,
    /// Name of the machine the tab runs on, shown as the entry detail.
    pub machine: &'a str,
    /// Kind of that machine, shown as the entry's target dot.
    pub target: TargetKind,
    /// The tab is the active one.
    pub is_active: bool,
}

/// One launch the palette can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteLaunch<'a> {
    /// Path in the launch tree: the folder titles, then the launch title.
    pub path: Vec<String>,
    /// Title of the launch, the entry's title.
    pub title: &'a str,
    /// The launch connects over SSH.
    pub is_ssh: bool,
}
