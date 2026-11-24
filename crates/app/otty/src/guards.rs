//! What input from the user does while a menu or a dialog is open, which
//! overlay lies on top of the window, and what decides whether a command can
//! run now. The input checks only see input: what follows from it, and what
//! arrives on its own, skips them (see `update_internal` in `update.rs`), so
//! it can not get lost.
//!
//! The overlays are the SSH secret prompt, the Settings
//! dialog, the notifications list, the command palette, the All modes menu,
//! the header menus, the pane menu of the terminal, the context menu and the name field
//! (the inline edit) of the Launches tree, the context menu, the name field
//! and the delete dialog of the Files tree, the create dialogs, the
//! confirmation dialog and the row menu of Containers mode, and the profile
//! dialog, the remove dialog, the statistics dialog, the New session dialog
//! and the menus of Agents mode, and the dialogs of Plugins mode.
//! A new dialog adds its answers to [`dialog_guard`]. A menu adds nothing:
//! what it sends comes marked as sent from it (see `screens::overlay_layer`)
//! and reaches it; a new kind of menu adds its trigger to
//! [`menu_opened_by`].

use otty_app_agents::{AgentsIntent, AgentsOverlay};
use otty_app_containers::{
    ActionsIntent, ContainersIntent, ContainersOverlay, CreateIntent,
};
use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_plugins::PluginsIntent;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_settings::SettingsIntent;
use otty_app_slots::OverlayKind;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::frame::title_bar::TitleBarIntent;
use crate::message::AppEvent;

/// An overlay: a menu, a dialog or the inline edit that lies over the window
/// and takes the user's input before the window under it. [`top_overlay`]
/// finds the one on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    /// The SSH prompt that asks for a password or a passphrase, over
    /// everything.
    SecretPrompt,
    /// The Settings dialog, over every mode.
    Settings,
    /// The notifications list under the bell, over every mode.
    NotificationsList,
    /// The command palette, over every mode.
    Palette,
    /// The All modes menu.
    ModesMenu,
    /// The New session or Launches menu of a section header.
    HeaderMenu,
    /// The context menu of a terminal pane, in Terminal mode.
    TerminalContextMenu,
    /// The context menu of the Launches tree, in Terminal mode.
    LaunchContextMenu,
    /// A name typed in the Launches tree: a new folder or a rename.
    InlineEdit,
    /// The context menu of the Files tree, in Terminal mode.
    FilesContextMenu,
    /// The dialog of the Files tree that asks before a deletion, in
    /// Terminal mode.
    FilesDeleteDialog,
    /// A name typed in the Files tree: a new file or folder, or a rename.
    FilesInlineEdit,
    /// A create dialog of Containers mode, such as Pull image.
    ContainersCreate,
    /// The dialog of Containers mode that asks to confirm a destructive
    /// action.
    ContainersConfirm,
    /// The menu of a row in Containers mode.
    ContainersRowMenu,
    /// The dialog of Agents mode that adds or edits a profile.
    AgentsProfileDialog,
    /// The dialog of Agents mode that adds an account by OTTY's own
    /// login.
    AgentsAddAccountDialog,
    /// The dialog of Agents mode of a key a plugin part asks for.
    AgentsKeyDialog,
    /// The dialog of Agents mode that names an account or a profile.
    AgentsRenameDialog,
    /// The dialog of Agents mode that asks before a profile is removed.
    AgentsRemoveDialog,
    /// The dialog of Agents mode that asks to allow the read of the system
    /// keyring.
    AgentsKeyringDialog,
    /// The statistics dialog of a session or a project in Agents mode.
    AgentsStatsDialog,
    /// The New session dialog of Agents mode.
    AgentsNewSessionDialog,
    /// The Found by OTTY report of the first start, in Agents mode.
    AgentsFoundReport,
    /// A menu of Agents mode: the funnel menu of the projects, the
    /// profiles of New session, the menu of a session, of a profile or of
    /// an account, or the "⋯" of All accounts or All profiles.
    AgentsMenu,
    /// A dialog of Plugins mode: Add plugin…, the install or update
    /// dialog, the log of a plugin or the question before Remove.
    PluginsDialog,
    /// The menu of a plugin in the left panel of Plugins mode.
    PluginsMenu,
}

impl Overlay {
    /// Return whether the overlay is a menu or a dialog. A click outside a
    /// menu closes it, and a shortcut closes it and runs. A dialog waits for
    /// its answer (see [`dialog_guard`]). The inline edit is neither: it is
    /// a field in the tree, not a layer over the terminals, and any other
    /// input cancels it (see [`inline_edit_guard`] and
    /// [`files_inline_edit_guard`]).
    pub(crate) fn kind(self) -> Option<OverlayKind> {
        match self {
            Overlay::SecretPrompt
            | Overlay::Settings
            | Overlay::FilesDeleteDialog
            | Overlay::ContainersCreate
            | Overlay::ContainersConfirm
            | Overlay::AgentsProfileDialog
            | Overlay::AgentsAddAccountDialog
            | Overlay::AgentsKeyDialog
            | Overlay::AgentsRenameDialog
            | Overlay::AgentsRemoveDialog
            | Overlay::AgentsKeyringDialog
            | Overlay::AgentsStatsDialog
            | Overlay::AgentsNewSessionDialog
            | Overlay::AgentsFoundReport
            | Overlay::PluginsDialog => Some(OverlayKind::Dialog),
            Overlay::NotificationsList
            | Overlay::Palette
            | Overlay::ModesMenu
            | Overlay::HeaderMenu
            | Overlay::TerminalContextMenu
            | Overlay::LaunchContextMenu
            | Overlay::FilesContextMenu
            | Overlay::ContainersRowMenu
            | Overlay::AgentsMenu
            | Overlay::PluginsMenu => Some(OverlayKind::Menu),
            Overlay::InlineEdit | Overlay::FilesInlineEdit => None,
        }
    }

    /// Return what Esc sends while this overlay is on top: the event that
    /// closes a menu, or Cancel of a dialog and of the inline edit.
    pub(crate) fn escape_event(self) -> AppEvent {
        match self {
            Overlay::SecretPrompt => {
                AppEvent::QuickLaunch(QuickLaunchIntent::SecretPromptCancel)
            },
            // Closing Settings drops the edits that were not saved.
            Overlay::Settings => AppEvent::Settings(SettingsIntent::Close),
            Overlay::NotificationsList => {
                AppEvent::Notifications(NotificationsIntent::CloseList)
            },
            Overlay::Palette => AppEvent::Palette(PaletteIntent::Dismiss),
            Overlay::ModesMenu => AppEvent::Modes(ModesIntent::CloseMenu),
            Overlay::HeaderMenu => {
                AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss)
            },
            Overlay::TerminalContextMenu => {
                AppEvent::Terminal(TerminalIntent::CloseAllContextMenus)
            },
            Overlay::LaunchContextMenu => {
                AppEvent::QuickLaunch(QuickLaunchIntent::ContextMenuDismiss)
            },
            Overlay::InlineEdit => {
                AppEvent::QuickLaunch(QuickLaunchIntent::CancelInlineEdit)
            },
            Overlay::FilesContextMenu => {
                AppEvent::Explorer(ExplorerIntent::ContextMenuDismiss)
            },
            Overlay::FilesDeleteDialog => {
                AppEvent::Explorer(ExplorerIntent::DeleteCancelled)
            },
            Overlay::FilesInlineEdit => {
                AppEvent::Explorer(ExplorerIntent::CancelInlineEdit)
            },
            Overlay::ContainersCreate => AppEvent::Containers(
                ContainersIntent::Create(CreateIntent::Cancelled),
            ),
            Overlay::ContainersConfirm => AppEvent::Containers(
                ContainersIntent::Actions(ActionsIntent::ConfirmCancelled),
            ),
            Overlay::ContainersRowMenu => AppEvent::Containers(
                ContainersIntent::Actions(ActionsIntent::CloseRowMenu),
            ),
            Overlay::AgentsProfileDialog => {
                AppEvent::Agents(AgentsIntent::Dialog(
                    otty_app_agents::ProfileDialogIntent::Cancelled,
                ))
            },
            Overlay::AgentsAddAccountDialog => {
                AppEvent::Agents(AgentsIntent::AddAccount(
                    otty_app_agents::AddAccountIntent::Cancelled,
                ))
            },
            Overlay::AgentsKeyDialog => {
                AppEvent::Agents(AgentsIntent::KeyDialog(
                    otty_app_agents::KeyDialogIntent::Cancelled,
                ))
            },
            Overlay::AgentsRenameDialog => AppEvent::Agents(
                AgentsIntent::Rename(otty_app_agents::RenameIntent::Cancelled),
            ),
            Overlay::AgentsRemoveDialog => {
                AppEvent::Agents(AgentsIntent::RemoveCancelled)
            },
            Overlay::AgentsKeyringDialog => {
                AppEvent::Agents(AgentsIntent::KeyringNotNow)
            },
            Overlay::AgentsStatsDialog => {
                AppEvent::Agents(AgentsIntent::CloseStats)
            },
            Overlay::AgentsNewSessionDialog => {
                AppEvent::Agents(AgentsIntent::NewSessionDialog(
                    otty_app_agents::NewSessionDialogIntent::Cancelled,
                ))
            },
            Overlay::AgentsFoundReport => {
                AppEvent::Agents(AgentsIntent::CloseFoundReport)
            },
            Overlay::AgentsMenu => AppEvent::Agents(AgentsIntent::CloseMenu),
            Overlay::PluginsDialog => AppEvent::Plugins(PluginsIntent::Back),
            Overlay::PluginsMenu => AppEvent::Plugins(PluginsIntent::CloseMenu),
        }
    }
}

/// Determines how the event loop should treat input from the user when a
/// menu or a dialog is on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuGuard {
    /// Let the event pass through to normal dispatch.
    Allow,
    /// Silently drop the event without closing the menu.
    Ignore,
    /// Close all menus; the event itself does nothing else.
    Dismiss,
    /// Close all menus, then dispatch the event.
    DismissAndRun,
}

/// Classify input from the user while the menu `top` is on top. What the
/// menu itself sends, its items, a hover or a scroll on it, comes marked
/// as [`AppEvent::FromMenu`] and passes before this check, so no item is
/// listed here. Everything else is input outside the menu.
///
/// A press on the trigger of another menu closes the open menus and opens
/// that one, with one press; a trigger of the menu on top goes to its
/// feature, which opens its other menu there or closes the same one (see
/// [`menu_opened_by`]). The bell and the palette button close their own
/// menu. A pointer move, a window control and what a
/// terminal reports leave the menus open. A shortcut, a tab press and
/// Settings close them and run. Other keys, a press on a pane and one in
/// the Launches tree do nothing; any other press, also one no widget takes
/// ([`AppEvent::PressedAway`]), only closes them.
pub(crate) fn context_menu_guard(top: Overlay, event: &AppEvent) -> MenuGuard {
    use MenuGuard::*;

    if let Some(opened) = menu_opened_by(event) {
        return match event {
            // The bell and the palette button close their own menu.
            AppEvent::TitleBar(_) if opened == top => Dismiss,
            _ if opened == top => Allow,
            _ => DismissAndRun,
        };
    }
    if keeps_menus_open(event) {
        return Allow;
    }

    match event {
        // A press on a pane comes with the right press that opens the pane
        // menu, and a press in the Launches tree with the right press that
        // opens its menu: they do nothing under a menu.
        AppEvent::Terminal(TerminalIntent::PaneClicked { .. })
        | AppEvent::QuickLaunch(_) => Ignore,
        // Opening settings closes the menus and goes on; the other header
        // buttons only close them.
        AppEvent::TitleBar(TitleBarIntent::OpenSettings) => DismissAndRun,
        // Choosing a mode closes every menu.
        AppEvent::Modes(ModesIntent::Select(_)) => DismissAndRun,
        // A press on a Sessions row, the New session button or the start
        // card closes the menus and runs. The cross of a row and a middle
        // press on it only close the menus: the tab stays.
        AppEvent::Tabs(TabsIntent::CloseTab { .. }) => Dismiss,
        AppEvent::Tabs(_) => DismissAndRun,
        // A shortcut closes the menus and runs. Other keys no widget took
        // do nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// The menu `event` opens, when it is the trigger of a menu: a "⋯" or a
/// drop-down of Agents mode, a right press on a row of a tree, a table or
/// a pane, the chevron of a section header, the "⋯" of the modes, the bell
/// or the palette button. A new menu adds its trigger here, or it needs a
/// second press while another menu is open.
pub(crate) fn menu_opened_by(event: &AppEvent) -> Option<Overlay> {
    let opened = match event {
        AppEvent::Agents(
            AgentsIntent::OpenSessionMenu(_)
            | AgentsIntent::OpenProfileMenu(_)
            | AgentsIntent::OpenAccountMenu(_)
            | AgentsIntent::OpenAllAccountsMenu
            | AgentsIntent::OpenAllProfilesMenu
            | AgentsIntent::OpenPeriodMenu
            | AgentsIntent::OpenFilterMenu
            | AgentsIntent::OpenNewSessionMenu,
        ) => Overlay::AgentsMenu,
        AppEvent::Plugins(PluginsIntent::OpenMenu(_)) => Overlay::PluginsMenu,
        AppEvent::Containers(
            ContainersIntent::OpenRowMenu { .. }
            | ContainersIntent::RowMenuPressed { .. }
            | ContainersIntent::Actions(ActionsIntent::OpenRowMenu { .. }),
        ) => Overlay::ContainersRowMenu,
        AppEvent::QuickLaunch(
            QuickLaunchIntent::NodeRightClicked { .. }
            | QuickLaunchIntent::BackgroundRightClicked,
        ) => Overlay::LaunchContextMenu,
        AppEvent::Explorer(
            ExplorerIntent::NodeRightClicked { .. }
            | ExplorerIntent::BackgroundRightClicked,
        ) => Overlay::FilesContextMenu,
        AppEvent::Terminal(TerminalIntent::OpenContextMenu { .. }) => {
            Overlay::TerminalContextMenu
        },
        AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(_)) => {
            Overlay::HeaderMenu
        },
        AppEvent::Modes(ModesIntent::ToggleMenu) => Overlay::ModesMenu,
        AppEvent::TitleBar(TitleBarIntent::OpenNotifications) => {
            Overlay::NotificationsList
        },
        AppEvent::TitleBar(TitleBarIntent::OpenCommandPalette) => {
            Overlay::Palette
        },
        _ => return None,
    };

    Some(opened)
}

/// Whether `event` leaves the open menus as they are: a pointer move over
/// the window, its header, a tree, the panes or the rows of a usage block,
/// a window control or grip, what a terminal reports, typing in the name
/// field of a tree, and a block command from the fold arrow or the sticky
/// header button of a block, which runs.
fn keeps_menus_open(event: &AppEvent) -> bool {
    #[cfg(not(target_os = "macos"))]
    if matches!(event, AppEvent::ResizeWindow(_)) {
        return true;
    }

    matches!(
        event,
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { .. })
            | AppEvent::TitleBar(
                TitleBarIntent::PressTitleBar
                    | TitleBarIntent::MoveOnTitleBar
                    | TitleBarIntent::ReleaseTitleBar
                    | TitleBarIntent::MinimizeWindow
                    | TitleBarIntent::ToggleMaximize
                    | TitleBarIntent::CloseWindow
            )
            | AppEvent::QuickLaunch(
                QuickLaunchIntent::CursorMoved { .. }
                    | QuickLaunchIntent::NodeHovered { .. }
                    | QuickLaunchIntent::InlineEditChanged(_)
                    | QuickLaunchIntent::InlineEditSubmit
            )
            | AppEvent::Explorer(
                ExplorerIntent::CursorMoved { .. }
                    | ExplorerIntent::NodeHovered { .. }
                    | ExplorerIntent::InlineEditChanged(_)
                    | ExplorerIntent::InlineEditSubmit
            )
            | AppEvent::Terminal(
                TerminalIntent::Widget(_)
                    | TerminalIntent::PaneGridCursorMoved { .. }
                    | TerminalIntent::RunBlockCommand { .. }
            )
            | AppEvent::Agents(AgentsIntent::HoverUsageRow(_))
    )
}

/// Classify input from the user while `dialog` is on top: it returns only
/// `Allow` or `Ignore`. A dialog waits for its answer, so only its own
/// answers, pointer moves and window controls pass. Shortcuts, app buttons,
/// presses outside it and other keys do nothing, and a menu under the dialog
/// stays open.
///
/// Settings is the one dialog that closes on a press on the dimmed area
/// around it: the press sends its Close, one of its own answers, so it
/// passes here like the close cross.
pub(crate) fn dialog_guard(dialog: Overlay, event: &AppEvent) -> MenuGuard {
    if is_dialog_answer(dialog, event) || passes_every_dialog(event) {
        return MenuGuard::Allow;
    }

    MenuGuard::Ignore
}

/// Return the overlay on top of the window, if one is open.
///
/// The order is fixed, top first: the SSH prompt, Settings, the
/// notifications list, the palette, the All modes menu, the header menus,
/// then the overlays of the active mode: in Terminal mode the delete dialog
/// of Files, the pane menu of the terminal, the context menus of the
/// launches and of Files and the names typed in their trees, in Containers
/// mode a create dialog, the confirmation dialog and the row menu, in Agents
/// mode the profile dialog, the Rename dialog, the remove dialog, the
/// statistics dialog, the New session dialog, the Found by OTTY report and
/// a menu, in Plugins mode its dialog and the menu of a plugin. Those of
/// the other modes are not on screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
    if app.quick_launch.has_secret_prompt() {
        return Some(Overlay::SecretPrompt);
    }
    if app.settings.is_open() {
        return Some(Overlay::Settings);
    }
    if app.notifications.is_list_open() {
        return Some(Overlay::NotificationsList);
    }
    if app.palette.is_open() {
        return Some(Overlay::Palette);
    }
    if app.modes.is_menu_open() {
        return Some(Overlay::ModesMenu);
    }
    if app.sidebar.has_header_menu_open() {
        return Some(Overlay::HeaderMenu);
    }

    match app.modes.active() {
        Mode::Terminal => terminal_overlay(app),
        Mode::Containers => containers_overlay(app),
        Mode::Agents => agents_overlay(app),
        Mode::Plugins => plugins_overlay(app),
    }
}

/// Gather what decides whether a command can run now: the mode on screen,
/// the active tab and its focused pane. The input filter checks shortcuts
/// with it, and the palette lists and runs only the commands it allows (see
/// `AppCommand::is_available`).
pub(crate) fn command_ctx(app: &App) -> CommandCtx {
    CommandCtx {
        is_terminal_mode: app.modes.active() == Mode::Terminal,
        has_active_tab: app.tabs.active_tab_id().is_some(),
        has_focused_pane: app.terminal.focused_pane().is_some(),
        has_local_folder: app.terminal.focused_local_folder().is_some(),
    }
}

/// Return `true` when the inline edit of the Launches tree should be
/// cancelled before the input runs: anything but typing into its name
/// field, pointer moves, terminal events and key presses. A shortcut
/// cancels it: it arrives as a command.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunch(
            QuickLaunchIntent::InlineEditChanged(_)
            | QuickLaunchIntent::InlineEditSubmit,
        ) => false,
        _ => !leaves_inline_edits(event),
    }
}

/// Return `true` when the name field of the Files tree should be cancelled
/// before the input runs, by the same rule as [`inline_edit_guard`]: all
/// but typing into it, pointer moves, terminal events and key presses.
pub(crate) fn files_inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::Explorer(
            ExplorerIntent::InlineEditChanged(_)
            | ExplorerIntent::InlineEditSubmit,
        ) => false,
        _ => !leaves_inline_edits(event),
    }
}

/// Return `true` when `event` is text a terminal sends to its shell and a
/// menu or a dialog lies on top: the keys belong to the overlay, even if a
/// terminal still had keyboard focus. The inline edit lets them through: it
/// is no overlay over the terminals.
pub(crate) fn is_terminal_input_kept_out(
    top: Option<Overlay>,
    event: &AppEvent,
) -> bool {
    let covers_terminals = top.and_then(Overlay::kind).is_some();

    covers_terminals && is_terminal_input(event)
}

/// The open overlay of Containers mode on top, as the containers feature
/// stacks them.
pub(crate) fn containers_overlay(app: &App) -> Option<Overlay> {
    let overlay = match app.containers.top_overlay()? {
        ContainersOverlay::CreateDialog => Overlay::ContainersCreate,
        ContainersOverlay::ConfirmDialog => Overlay::ContainersConfirm,
        ContainersOverlay::RowMenu => Overlay::ContainersRowMenu,
    };

    Some(overlay)
}

/// The open overlay of Plugins mode on top: its dialog lies over the
/// menu of a plugin.
fn plugins_overlay(app: &App) -> Option<Overlay> {
    if app.plugins.has_dialog() {
        return Some(Overlay::PluginsDialog);
    }

    app.plugins.has_menu().then_some(Overlay::PluginsMenu)
}

/// The open overlay of Agents mode on top, as the agents feature stacks
/// them.
pub(crate) fn agents_overlay(app: &App) -> Option<Overlay> {
    let overlay = match app.agents.top_overlay()? {
        AgentsOverlay::ProfileDialog => Overlay::AgentsProfileDialog,
        AgentsOverlay::AddAccountDialog => Overlay::AgentsAddAccountDialog,
        AgentsOverlay::KeyDialog => Overlay::AgentsKeyDialog,
        AgentsOverlay::RenameDialog => Overlay::AgentsRenameDialog,
        AgentsOverlay::RemoveDialog => Overlay::AgentsRemoveDialog,
        AgentsOverlay::KeyringDialog => Overlay::AgentsKeyringDialog,
        AgentsOverlay::StatsDialog => Overlay::AgentsStatsDialog,
        AgentsOverlay::NewSessionDialog => Overlay::AgentsNewSessionDialog,
        AgentsOverlay::FoundReport => Overlay::AgentsFoundReport,
        AgentsOverlay::Menu => Overlay::AgentsMenu,
    };

    Some(overlay)
}

/// Whether `event` leaves a name being typed in a tree alone: a pointer
/// move or a hover over the window, the Launches tree or the Files tree, a
/// report of a terminal, a key press no widget took, or a press no widget
/// took.
fn leaves_inline_edits(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::QuickLaunch(
            QuickLaunchIntent::CursorMoved { .. }
                | QuickLaunchIntent::NodeHovered { .. }
        ) | AppEvent::Explorer(
            ExplorerIntent::CursorMoved { .. }
                | ExplorerIntent::NodeHovered { .. }
        ) | AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { .. })
            | AppEvent::Terminal(
                TerminalIntent::Widget(_)
                    | TerminalIntent::PaneGridCursorMoved { .. },
            )
            | AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar)
            | AppEvent::Keyboard(_)
            | AppEvent::PressedAway
    )
}

/// Whether `event` answers `dialog`: typing in it, its buttons and choices,
/// its close cross, Esc in its fields and, for Settings, a press on the
/// dimmed area around it. The Containers dialogs send nothing from their
/// dimmed area: a press there does nothing.
fn is_dialog_answer(dialog: Overlay, event: &AppEvent) -> bool {
    match dialog {
        Overlay::SecretPrompt => matches!(
            event,
            AppEvent::QuickLaunch(
                QuickLaunchIntent::SecretPromptChanged(_)
                    | QuickLaunchIntent::SecretPromptSubmit
                    | QuickLaunchIntent::SecretPromptCancel
            )
        ),
        Overlay::Settings => matches!(
            event,
            AppEvent::Settings(
                SettingsIntent::ThemeSelected(_)
                    | SettingsIntent::LanguageChanged(_)
                    | SettingsIntent::ShellChanged(_)
                    | SettingsIntent::EditorChanged(_)
                    | SettingsIntent::EqualizePanesToggled(_)
                    | SettingsIntent::Save
                    | SettingsIntent::Reset
                    | SettingsIntent::Close
            )
        ),
        // Every field and button of the dialogs of Plugins mode; the
        // answers of its I/O arrive on their own.
        Overlay::PluginsDialog => matches!(event, AppEvent::Plugins(_)),
        Overlay::FilesDeleteDialog => matches!(
            event,
            AppEvent::Explorer(
                ExplorerIntent::DeleteConfirmed
                    | ExplorerIntent::DeleteCancelled
            )
        ),
        Overlay::ContainersConfirm => matches!(
            event,
            AppEvent::Containers(ContainersIntent::Actions(
                ActionsIntent::ConfirmSwitchToggled(_)
                    | ActionsIntent::ConfirmAccepted
                    | ActionsIntent::ConfirmCancelled
            ))
        ),
        Overlay::ContainersCreate => matches!(
            event,
            AppEvent::Containers(ContainersIntent::Create(
                CreateIntent::Changed(..)
                    | CreateIntent::Submitted
                    | CreateIntent::Cancelled
            ))
        ),
        // Every field, choice and button of the profile dialog.
        Overlay::AgentsProfileDialog => {
            matches!(event, AppEvent::Agents(AgentsIntent::Dialog(_)))
        },
        // The select, Continue, Cancel and Manage plugins… of the Add
        // account dialog.
        Overlay::AgentsAddAccountDialog => {
            matches!(event, AppEvent::Agents(AgentsIntent::AddAccount(_)))
        },
        // Every field, choice and button of the key dialog.
        Overlay::AgentsKeyDialog => {
            matches!(event, AppEvent::Agents(AgentsIntent::KeyDialog(_)))
        },
        // The name field, Save, Cancel and Esc of the Rename dialog.
        Overlay::AgentsRenameDialog => {
            matches!(event, AppEvent::Agents(AgentsIntent::Rename(_)))
        },
        Overlay::AgentsRemoveDialog => matches!(
            event,
            AppEvent::Agents(
                AgentsIntent::RemoveConfirmed | AgentsIntent::RemoveCancelled
            )
        ),
        // Allow and Not now; the answer of the read arrives on its own.
        Overlay::AgentsKeyringDialog => matches!(
            event,
            AppEvent::Agents(
                AgentsIntent::AllowKeyring | AgentsIntent::KeyringNotNow
            )
        ),
        // Close, the cross and everything of its usage block: the list of
        // its period drop-down, its tabs and its rows. The answer of the read
        // arrives on its own, so it does not pass here.
        Overlay::AgentsStatsDialog => matches!(
            event,
            AppEvent::Agents(
                AgentsIntent::CloseStats
                    | AgentsIntent::OpenPeriodMenu
                    | AgentsIntent::PickPeriod(_)
                    | AgentsIntent::ShowUsageTab(_)
                    | AgentsIntent::PickUsageRow(_)
                    | AgentsIntent::HoverUsageRow(_)
            )
        ),
        // Every field, suggestion and button of the New session dialog.
        // The answer of the folder look-up arrives on its own.
        Overlay::AgentsNewSessionDialog => {
            matches!(event, AppEvent::Agents(AgentsIntent::NewSessionDialog(_)))
        },
        // Done and the button of each need.
        Overlay::AgentsFoundReport => matches!(
            event,
            AppEvent::Agents(
                AgentsIntent::CloseFoundReport
                    | AgentsIntent::FoundReportNeed(_)
            )
        ),
        _ => false,
    }
}

/// Whether `event` passes under any dialog: a pointer move, a window
/// control or a report of a terminal (its typed input is kept out before,
/// see [`is_terminal_input_kept_out`]). A hover over a Files row or a
/// Launches row passes too, so the tree under the dialog keeps its hover.
fn passes_every_dialog(event: &AppEvent) -> bool {
    #[cfg(not(target_os = "macos"))]
    if matches!(event, AppEvent::ResizeWindow(_)) {
        return true;
    }

    matches!(
        event,
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { .. })
            | AppEvent::Terminal(
                TerminalIntent::PaneGridCursorMoved { .. }
                    | TerminalIntent::Widget(_)
            )
            | AppEvent::Explorer(
                ExplorerIntent::CursorMoved { .. }
                    | ExplorerIntent::NodeHovered { .. }
            )
            | AppEvent::QuickLaunch(
                QuickLaunchIntent::CursorMoved { .. }
                    | QuickLaunchIntent::NodeHovered { .. }
            )
            | AppEvent::TitleBar(
                TitleBarIntent::ToggleMaximize
                    | TitleBarIntent::MinimizeWindow
                    | TitleBarIntent::CloseWindow
                    | TitleBarIntent::PressTitleBar
                    | TitleBarIntent::MoveOnTitleBar
                    | TitleBarIntent::ReleaseTitleBar
            )
    )
}

/// The open overlay of Terminal mode on top: the delete dialog of Files
/// lies over the menus, and a context menu over a name being typed.
fn terminal_overlay(app: &App) -> Option<Overlay> {
    if app.explorer.has_delete_dialog() {
        return Some(Overlay::FilesDeleteDialog);
    }
    if app.terminal.has_context_menu() {
        return Some(Overlay::TerminalContextMenu);
    }
    if app.quick_launch.has_context_menu() {
        return Some(Overlay::LaunchContextMenu);
    }
    if app.explorer.has_context_menu() {
        return Some(Overlay::FilesContextMenu);
    }
    if app.quick_launch.has_inline_edit() {
        return Some(Overlay::InlineEdit);
    }
    if app.explorer.has_inline_edit() {
        return Some(Overlay::FilesInlineEdit);
    }

    None
}

/// Return `true` when the event carries text a terminal sends to its shell:
/// typed keys, a paste or composed input.
fn is_terminal_input(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::Write { .. }
        ))
    )
}

#[cfg(test)]
mod tests {
    use iced::keyboard::Modifiers;
    use iced::keyboard::key::{Code, Physical};
    use otty_app_agents::{
        AddAccountIntent, AgentsCtx, FolderFieldIntent, KeyDialogIntent,
        NewSessionDialogIntent, ProfileDialogIntent, RenameIntent, UsagePeriod,
        UsageTab,
    };
    use otty_app_domain::commands::AppCommand;
    use otty_app_explorer::ExplorerIntent;
    use otty_app_i18n as i18n;
    use otty_app_slots::Component;
    use otty_ui_term::BlockKey;

    use super::*;
    use crate::frame::sidebar::types::HeaderMenu;
    use crate::testing::agents::{
        PROJECT, codex_profile, first_start_profiles_loaded,
        one_session_loaded, profiles_loaded,
    };
    use crate::testing::keyboard::key_press;
    use crate::testing::running::RunningApp;
    use crate::testing::window::{
        app, containers_app_with_lists, open_confirm, open_create_dialog,
        open_files_delete_dialog, open_files_menu, open_launch_menu,
        open_notification_list, open_palette, open_pane_menu, open_row_menu,
        open_secret_prompt, open_settings, open_terminal_tab,
        start_files_inline_edit, start_inline_edit,
    };

    /// Opens one overlay on an app.
    type Opener = fn(&mut App);

    fn open_modes_menu(app: &mut App) {
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
    }

    fn open_header_menu(app: &mut App) {
        let _ = app
            .sidebar
            .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions), ());
    }

    /// The openers of the overlays, with the overlay each one opens.
    const OVERLAYS: [(Opener, Overlay); 9] = [
        (open_secret_prompt, Overlay::SecretPrompt),
        (open_settings, Overlay::Settings),
        (open_notification_list, Overlay::NotificationsList),
        (open_palette, Overlay::Palette),
        (open_modes_menu, Overlay::ModesMenu),
        (open_header_menu, Overlay::HeaderMenu),
        (open_pane_menu, Overlay::TerminalContextMenu),
        (open_launch_menu, Overlay::LaunchContextMenu),
        (start_inline_edit, Overlay::InlineEdit),
    ];

    /// The openers of the overlays of Containers mode, with the overlay each
    /// one opens. They need the lists of [`containers_app_with_lists`].
    const CONTAINERS_OVERLAYS: [(Opener, Overlay); 3] = [
        (open_create_dialog, Overlay::ContainersCreate),
        (open_confirm, Overlay::ContainersConfirm),
        (open_row_menu, Overlay::ContainersRowMenu),
    ];

    /// The openers of the overlays of Agents mode, with the overlay each
    /// one opens. They need the profile of [`agents_app_with_a_profile`].
    const AGENTS_OVERLAYS: [(Opener, Overlay); 10] = [
        (open_profile_dialog, Overlay::AgentsProfileDialog),
        (open_add_account_dialog, Overlay::AgentsAddAccountDialog),
        (open_key_dialog, Overlay::AgentsKeyDialog),
        (open_rename_dialog, Overlay::AgentsRenameDialog),
        (open_remove_dialog, Overlay::AgentsRemoveDialog),
        (open_keyring_dialog, Overlay::AgentsKeyringDialog),
        (open_stats_dialog, Overlay::AgentsStatsDialog),
        (open_new_session_dialog, Overlay::AgentsNewSessionDialog),
        (open_found_report, Overlay::AgentsFoundReport),
        (open_agents_menu, Overlay::AgentsMenu),
    ];

    /// The dialogs: they wait for their answer.
    const DIALOGS: [Overlay; 13] = [
        Overlay::SecretPrompt,
        Overlay::Settings,
        Overlay::ContainersCreate,
        Overlay::ContainersConfirm,
        Overlay::AgentsProfileDialog,
        Overlay::AgentsAddAccountDialog,
        Overlay::AgentsKeyDialog,
        Overlay::AgentsRenameDialog,
        Overlay::AgentsRemoveDialog,
        Overlay::AgentsKeyringDialog,
        Overlay::AgentsStatsDialog,
        Overlay::AgentsNewSessionDialog,
        Overlay::AgentsFoundReport,
    ];

    #[test]
    fn each_overlay_is_a_menu_a_dialog_or_the_inline_edit() {
        for (_, overlay) in OVERLAYS
            .into_iter()
            .chain(CONTAINERS_OVERLAYS)
            .chain(AGENTS_OVERLAYS)
        {
            let expected = if DIALOGS.contains(&overlay) {
                Some(OverlayKind::Dialog)
            } else if overlay == Overlay::InlineEdit {
                None
            } else {
                Some(OverlayKind::Menu)
            };

            assert_eq!(overlay.kind(), expected, "{overlay:?}");
        }
    }

    /// Scenario 14 of the old app: an SSH launch asks for its password
    /// while a menu is open, in either mode.
    #[test]
    fn the_secret_prompt_lies_over_an_open_menu() {
        let menus: [Opener; 6] = [
            open_notification_list,
            open_palette,
            open_modes_menu,
            open_header_menu,
            open_pane_menu,
            open_launch_menu,
        ];

        for open_menu in menus {
            let mut app = app();
            open_menu(&mut app);
            open_secret_prompt(&mut app);

            assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
        }
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_secret_prompt(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
    }

    /// An SSH launch asks for its password while the row menu or a dialog of
    /// Containers mode is open: the prompt lies over them.
    #[test]
    fn the_secret_prompt_lies_over_the_overlays_of_containers_mode() {
        for (open, overlay) in CONTAINERS_OVERLAYS {
            let mut app = containers_app_with_lists();
            open(&mut app);
            open_secret_prompt(&mut app);

            assert_eq!(
                top_overlay(&app),
                Some(Overlay::SecretPrompt),
                "{overlay:?}"
            );
        }
    }

    /// The palette lies over the row menu, the notifications list over the
    /// palette and Settings over both, in Containers mode too.
    #[test]
    fn the_frame_overlays_lie_over_the_overlays_of_containers_mode() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);
        open_palette(&mut app);
        let over_the_row_menu = top_overlay(&app);
        open_notification_list(&mut app);
        let over_the_palette = top_overlay(&app);

        open_settings(&mut app);

        assert_eq!(over_the_row_menu, Some(Overlay::Palette));
        assert_eq!(over_the_palette, Some(Overlay::NotificationsList));
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// Scenario 11 of the old app: a Containers dialog stays open, hidden,
    /// while Terminal mode is shown, and comes back with its mode.
    #[test]
    fn a_containers_overlay_counts_only_in_containers_mode() {
        let mut app = containers_app_with_lists();
        open_confirm(&mut app);

        let _ = app.modes.reduce(ModesIntent::Select(Mode::Terminal), ());
        let in_terminal = top_overlay(&app);
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        assert_eq!(in_terminal, None);
        assert_eq!(top_overlay(&app), Some(Overlay::ContainersConfirm));
    }

    /// A dialog of Containers mode lies over the row menu it came from.
    #[test]
    fn a_containers_dialog_lies_over_the_row_menu() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);
        open_create_dialog(&mut app);

        let over_the_menu = top_overlay(&app);
        let _ = app
            .containers
            .reduce(ContainersIntent::Create(CreateIntent::Cancelled), ());

        assert_eq!(over_the_menu, Some(Overlay::ContainersCreate));
        assert_eq!(
            top_overlay(&app),
            Some(Overlay::ContainersRowMenu),
            "the row menu waits under the dialog"
        );
    }

    /// An SSH launch asks for its password while Settings is open.
    #[test]
    fn the_secret_prompt_lies_over_settings() {
        let mut app = app();
        open_settings(&mut app);
        open_secret_prompt(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
    }

    /// The context menu of the launches belongs to Terminal mode: in
    /// Containers mode it is not on screen.
    #[test]
    fn the_launch_menu_counts_only_in_terminal_mode() {
        let mut app = app();
        open_launch_menu(&mut app);

        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        assert_eq!(top_overlay(&app), None);
    }

    /// A name typed in the tree stays while another overlay opens over it.
    #[test]
    fn the_inline_edit_is_under_every_other_overlay() {
        let others = OVERLAYS
            .into_iter()
            .filter(|(_, overlay)| *overlay != Overlay::InlineEdit);

        for (open, overlay) in others {
            let mut app = app();
            start_inline_edit(&mut app);
            open(&mut app);

            assert!(app.quick_launch.has_inline_edit(), "{overlay:?}");
            assert_eq!(top_overlay(&app), Some(overlay));
        }
    }

    /// Settings lies over every menu, and counts in Containers mode too.
    #[test]
    fn settings_lies_over_every_menu_in_either_mode() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = app.modes.reduce(ModesIntent::Select(mode), ());
            open_header_menu(&mut app);
            open_modes_menu(&mut app);
            open_palette(&mut app);
            open_notification_list(&mut app);

            open_settings(&mut app);

            assert_eq!(top_overlay(&app), Some(Overlay::Settings), "{mode:?}");
        }
    }

    fn settings(intent: SettingsIntent) -> AppEvent {
        AppEvent::Settings(intent)
    }

    /// Each dialog with what the user answers it: typing, its choices, its
    /// buttons, the close cross and Esc in a field.
    fn dialogs_with_answers() -> [(Overlay, Vec<AppEvent>); 13] {
        use otty_app_containers::FormField;
        use otty_app_i18n::Locale;
        use otty_app_settings::LanguageSetting;
        use otty_app_theme::theme::BuiltinTheme;

        let quick_launch = AppEvent::QuickLaunch;
        let confirm =
            |intent| AppEvent::Containers(ContainersIntent::Actions(intent));
        let create =
            |intent| AppEvent::Containers(ContainersIntent::Create(intent));

        [
            (
                Overlay::SecretPrompt,
                vec![
                    quick_launch(QuickLaunchIntent::SecretPromptChanged(
                        String::from("hunter2"),
                    )),
                    quick_launch(QuickLaunchIntent::SecretPromptSubmit),
                    quick_launch(QuickLaunchIntent::SecretPromptCancel),
                ],
            ),
            (
                Overlay::Settings,
                vec![
                    settings(SettingsIntent::ThemeSelected(
                        BuiltinTheme::OttyLight,
                    )),
                    settings(SettingsIntent::LanguageChanged(
                        LanguageSetting::Fixed(Locale::En),
                    )),
                    settings(SettingsIntent::ShellChanged(String::from(
                        "/bin/zsh",
                    ))),
                    settings(SettingsIntent::EditorChanged(String::from(
                        "nvim",
                    ))),
                    settings(SettingsIntent::EqualizePanesToggled(false)),
                    settings(SettingsIntent::Save),
                    settings(SettingsIntent::Reset),
                    settings(SettingsIntent::Close),
                ],
            ),
            (
                Overlay::ContainersConfirm,
                vec![
                    confirm(ActionsIntent::ConfirmSwitchToggled(true)),
                    confirm(ActionsIntent::ConfirmAccepted),
                    confirm(ActionsIntent::ConfirmCancelled),
                ],
            ),
            (
                Overlay::ContainersCreate,
                vec![
                    create(CreateIntent::Changed(
                        FormField::Name,
                        String::from("nginx"),
                    )),
                    create(CreateIntent::Submitted),
                    create(CreateIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsProfileDialog,
                vec![
                    profile_dialog(ProfileDialogIntent::NameChanged(
                        String::from("Semrush"),
                    )),
                    profile_dialog(ProfileDialogIntent::FolderChoicePicked(
                        otty_app_agents::FolderChoice::Typed,
                    )),
                    profile_folder(FolderFieldIntent::Changed(String::from(
                        "~/.claude-semrush",
                    ))),
                    profile_folder(FolderFieldIntent::SuggestionPicked(0)),
                    profile_folder(FolderFieldIntent::SuggestionHighlighted(
                        Some(0),
                    )),
                    profile_folder(FolderFieldIntent::SuggestionsClosed),
                    profile_dialog(ProfileDialogIntent::Submitted),
                    profile_dialog(ProfileDialogIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsAddAccountDialog,
                vec![
                    add_account(AddAccountIntent::ProviderListToggled),
                    add_account(AddAccountIntent::ProviderPicked(0)),
                    add_account(AddAccountIntent::Continued),
                    add_account(AddAccountIntent::PluginsOpened),
                    add_account(AddAccountIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsKeyDialog,
                vec![
                    key_dialog(KeyDialogIntent::KeyChoicePicked(
                        otty_app_agents::KeyPlaceChoice::Variable,
                    )),
                    key_dialog(KeyDialogIntent::KeyChanged(String::from(
                        "EXAMPLE_ADMIN_KEY",
                    ))),
                    key_dialog(KeyDialogIntent::PageOpened),
                    key_dialog(KeyDialogIntent::Submitted),
                    key_dialog(KeyDialogIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsRenameDialog,
                vec![
                    rename(RenameIntent::NameChanged(String::from("Work"))),
                    rename(RenameIntent::Submitted),
                    rename(RenameIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsRemoveDialog,
                vec![
                    AppEvent::Agents(AgentsIntent::RemoveConfirmed),
                    AppEvent::Agents(AgentsIntent::RemoveCancelled),
                ],
            ),
            (
                Overlay::AgentsKeyringDialog,
                vec![
                    AppEvent::Agents(AgentsIntent::AllowKeyring),
                    AppEvent::Agents(AgentsIntent::KeyringNotNow),
                ],
            ),
            (
                Overlay::AgentsStatsDialog,
                vec![
                    AppEvent::Agents(AgentsIntent::CloseStats),
                    AppEvent::Agents(AgentsIntent::OpenPeriodMenu),
                    AppEvent::Agents(AgentsIntent::PickPeriod(
                        UsagePeriod::Today,
                    )),
                    AppEvent::Agents(AgentsIntent::ShowUsageTab(
                        UsageTab::ByProfile,
                    )),
                    AppEvent::Agents(AgentsIntent::PickUsageRow(String::from(
                        "Codex",
                    ))),
                    AppEvent::Agents(AgentsIntent::HoverUsageRow(None)),
                ],
            ),
            (
                Overlay::AgentsNewSessionDialog,
                vec![
                    new_session_folder(FolderFieldIntent::Changed(
                        String::from("~/projects"),
                    )),
                    new_session_folder(FolderFieldIntent::SuggestionPicked(0)),
                    new_session_folder(
                        FolderFieldIntent::SuggestionHighlighted(Some(0)),
                    ),
                    new_session_folder(FolderFieldIntent::SuggestionsClosed),
                    new_session_dialog(NewSessionDialogIntent::ProfilePicked(
                        work_profile(),
                    )),
                    new_session_dialog(NewSessionDialogIntent::Submitted),
                    new_session_dialog(NewSessionDialogIntent::Cancelled),
                ],
            ),
            (
                Overlay::AgentsFoundReport,
                vec![
                    AppEvent::Agents(AgentsIntent::CloseFoundReport),
                    AppEvent::Agents(AgentsIntent::FoundReportNeed(
                        otty_app_agents::NeedAction::Run {
                            title: String::from("Work"),
                            command: otty_agents::AgentCommand::new("codex")
                                .with_arg("login"),
                        },
                    )),
                ],
            ),
        ]
    }

    fn new_session_dialog(intent: NewSessionDialogIntent) -> AppEvent {
        AppEvent::Agents(AgentsIntent::NewSessionDialog(intent))
    }

    fn profile_dialog(intent: ProfileDialogIntent) -> AppEvent {
        AppEvent::Agents(AgentsIntent::Dialog(intent))
    }

    fn new_session_folder(intent: FolderFieldIntent) -> AppEvent {
        new_session_dialog(NewSessionDialogIntent::Folder(intent))
    }

    fn add_account(intent: AddAccountIntent) -> AppEvent {
        AppEvent::Agents(AgentsIntent::AddAccount(intent))
    }

    fn key_dialog(intent: KeyDialogIntent) -> AppEvent {
        AppEvent::Agents(AgentsIntent::KeyDialog(intent))
    }

    fn rename(intent: RenameIntent) -> AppEvent {
        AppEvent::Agents(AgentsIntent::Rename(intent))
    }

    fn profile_folder(intent: FolderFieldIntent) -> AppEvent {
        profile_dialog(ProfileDialogIntent::Folder(intent))
    }

    /// An app in Agents mode with the added profile Codex · Work.
    fn agents_app_with_a_profile() -> App {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Agents), ());
        let _ = app.agents.reduce(
            profiles_loaded(vec![(
                work_profile(),
                crate::testing::agents::not_logged_in(),
            )]),
            AgentsCtx::default(),
        );

        app
    }

    fn work_profile() -> otty_agents::Profile {
        otty_agents::Profile::added(
            otty_agents::AgentId::new("codex"),
            "Codex · Work",
            std::path::PathBuf::from("/home/user/.codex-work"),
        )
    }

    fn open_profile_dialog(app: &mut App) {
        let _ = app
            .agents
            .reduce(AgentsIntent::OpenAddProfile, AgentsCtx::default());
    }

    /// Rename… of the menu of Codex · Work.
    fn open_rename_dialog(app: &mut App) {
        for intent in [
            AgentsIntent::OpenProfileMenu(work_profile()),
            AgentsIntent::ProfileMenu(otty_app_agents::ProfileAction::Rename),
        ] {
            let _ = app.agents.reduce(intent, AgentsCtx::default());
        }
    }

    /// The `+` of the Accounts section.
    fn open_add_account_dialog(app: &mut App) {
        let _ = app
            .agents
            .reduce(AgentsIntent::OpenAddAccount, AgentsCtx::default());
    }

    /// The button of a need that asks for the admin key of the bill of an
    /// organization.
    fn open_key_dialog(app: &mut App) {
        let organization = otty_agents::Account::new(
            "example",
            otty_agents::ProviderId::new("example-cloud"),
            "org:made-up-org",
            "Made-up org",
            otty_agents::AccountKind::Organization,
            Vec::new(),
        );
        let ask = otty_agents::KeyAsk::new(
            "admin",
            "Admin key",
            "Reads the real bill.",
        );

        let _ = app.agents.reduce(
            AgentsIntent::FixNeed(otty_app_agents::NeedAction::AskKey {
                account: Box::new(organization),
                plugin: String::from("Example"),
                ask,
            }),
            AgentsCtx::default(),
        );
    }

    fn open_remove_dialog(app: &mut App) {
        let _ = app.agents.reduce(
            AgentsIntent::RemoveProfile(work_profile()),
            AgentsCtx::default(),
        );
    }

    /// Allow keyring of the need of the system keyring.
    fn open_keyring_dialog(app: &mut App) {
        let _ = app.agents.reduce(
            AgentsIntent::FixNeed(otty_app_agents::NeedAction::AllowKeyring),
            AgentsCtx::default(),
        );
    }

    /// The statistics of the project of a session of Codex · Work.
    fn open_stats_dialog(app: &mut App) {
        for intent in [
            one_session_loaded(work_profile(), "w1"),
            AgentsIntent::SelectProject(std::path::PathBuf::from(PROJECT)),
            AgentsIntent::ShowProjectStats,
        ] {
            let _ = app.agents.reduce(intent, AgentsCtx::default());
        }
    }

    fn open_new_session_dialog(app: &mut App) {
        let _ = app.agents.reduce(
            AgentsIntent::OpenNewSessionDialog(None),
            AgentsCtx::default(),
        );
    }

    /// A read of the profiles on the first start opens Found by OTTY.
    fn open_found_report(app: &mut App) {
        let _ = app.agents.reduce(
            first_start_profiles_loaded(vec![(
                work_profile(),
                crate::testing::agents::not_logged_in(),
            )]),
            AgentsCtx::default(),
        );
    }

    fn open_agents_menu(app: &mut App) {
        let _ = app
            .agents
            .reduce(AgentsIntent::OpenFilterMenu, AgentsCtx::default());
    }

    /// The Agents dialogs and menus are found in Agents mode only, and Esc
    /// cancels or closes them.
    #[test]
    fn the_agents_overlays_count_in_agents_mode_and_esc_cancels_them() {
        for (open, overlay) in AGENTS_OVERLAYS {
            let mut app = agents_app_with_a_profile();
            open(&mut app);
            let found = top_overlay(&app);
            let _ = app.modes.reduce(ModesIntent::Select(Mode::Terminal), ());

            assert_eq!(found, Some(overlay));
            assert_eq!(top_overlay(&app), None, "{overlay:?}");
            assert!(matches!(overlay.escape_event(), AppEvent::Agents(_)));
        }
    }

    #[test]
    fn a_dialog_ignores_the_answers_of_another_dialog() {
        let all = dialogs_with_answers();

        for (dialog, _) in &all {
            let others = all.iter().filter(|(other, _)| other != dialog);
            for (_, answers) in others {
                for answer in answers {
                    assert!(!is_allowed(*dialog, answer), "{dialog:?}");
                }
            }
        }
    }

    /// The Containers dialogs send nothing from their dimmed area, and a
    /// press on the table or the tree under them is no answer: the dialog
    /// stays open.
    #[test]
    fn a_containers_dialog_ignores_a_press_outside_it() {
        let presses = [
            ContainersIntent::SelectRow(0),
            ContainersIntent::Refresh,
            ContainersIntent::Actions(ActionsIntent::Prune),
            ContainersIntent::Actions(ActionsIntent::CloseRowMenu),
        ]
        .map(AppEvent::Containers);

        for dialog in [Overlay::ContainersConfirm, Overlay::ContainersCreate] {
            for press in &presses {
                assert!(!is_allowed(dialog, press), "{dialog:?}: {press:?}");
            }
        }
    }

    fn is_allowed(dialog: Overlay, event: &AppEvent) -> bool {
        dialog_guard(dialog, event) == MenuGuard::Allow
    }

    #[test]
    fn a_dialog_lets_its_own_answers_through() {
        for (dialog, answers) in dialogs_with_answers() {
            for answer in &answers {
                assert!(is_allowed(dialog, answer), "{dialog:?}: {answer:?}");
            }
        }
    }

    /// Only the file answers and the open come from elsewhere: they are no
    /// answers of the user, so input never sends them.
    #[test]
    fn settings_ignores_what_the_user_can_not_send_it() {
        let others = [
            settings(SettingsIntent::Open),
            settings(SettingsIntent::ReloadFailed(String::from("no access"))),
            settings(SettingsIntent::SaveFailed(String::from("disk full"))),
        ];

        for event in &others {
            assert!(!is_allowed(Overlay::Settings, event), "{event:?}");
        }
    }

    /// Scenario 8 of the old app: a shortcut, a header button, a press
    /// outside the dialog and a plain key do nothing while a dialog waits
    /// for its answer. So the gear and the Settings shortcut do nothing
    /// while Settings is open (owner, Q3), and the bell, the title bar
    /// search, the palette shortcut and a toast's cross neither.
    #[test]
    fn a_dialog_ignores_shortcuts_app_buttons_and_keys() {
        let events = [
            AppEvent::Command(AppCommand::ToggleFocusMode),
            AppEvent::Command(AppCommand::OpenPalette),
            AppEvent::Command(AppCommand::OpenSettings),
            AppEvent::Command(AppCommand::OpenNotifications),
            AppEvent::Command(AppCommand::SelectModeAt { position: 0 }),
            AppEvent::TitleBar(TitleBarIntent::ToggleLeftPanel),
            AppEvent::TitleBar(TitleBarIntent::ToggleRightPanel),
            AppEvent::TitleBar(TitleBarIntent::ToggleFocusMode),
            AppEvent::TitleBar(TitleBarIntent::OpenSettings),
            AppEvent::TitleBar(TitleBarIntent::OpenCommandPalette),
            AppEvent::TitleBar(TitleBarIntent::OpenNotifications),
            AppEvent::Notifications(NotificationsIntent::DismissToast(1)),
            AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
            AppEvent::Modes(ModesIntent::ToggleMenu),
            AppEvent::Sidebar(SidebarIntent::ToggleRight),
            AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
                HeaderMenu::Sessions,
            )),
            AppEvent::Palette(PaletteIntent::Open),
            AppEvent::Explorer(ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            }),
            AppEvent::Tabs(TabsIntent::ActivateTab { tab_id: 1 }),
            AppEvent::Tabs(TabsIntent::OpenTerminalTab { folder: None }),
            AppEvent::Sidebar(SidebarIntent::ToggleSessions),
            AppEvent::Sidebar(SidebarIntent::ToggleLaunches),
            AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundPressed),
            AppEvent::QuickLaunch(QuickLaunchIntent::LaunchNode {
                path: vec![String::from("Demo")],
            }),
            AppEvent::Keyboard(key_press(
                Physical::Code(Code::KeyA),
                "a",
                Modifiers::empty(),
            )),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(!is_allowed(dialog, event), "{dialog:?}: {event:?}");
            }
        }
    }

    #[test]
    fn a_dialog_keeps_window_controls_and_pointer_moves() {
        let events = [
            AppEvent::TitleBar(TitleBarIntent::ToggleMaximize),
            AppEvent::TitleBar(TitleBarIntent::MinimizeWindow),
            AppEvent::TitleBar(TitleBarIntent::CloseWindow),
            AppEvent::TitleBar(TitleBarIntent::PressTitleBar),
            AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar),
            AppEvent::TitleBar(TitleBarIntent::ReleaseTitleBar),
            AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved {
                position: iced::Point::ORIGIN,
            }),
            // A row sends the path when the pointer enters it, and no path
            // when the pointer leaves it.
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
            AppEvent::Explorer(ExplorerIntent::NodeHovered { path: None }),
            // The pointer over the Launches tree and its rows.
            AppEvent::QuickLaunch(QuickLaunchIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            }),
            AppEvent::QuickLaunch(QuickLaunchIntent::NodeHovered {
                path: vec![String::from("Demo")],
            }),
            // What a terminal under the dialog reports: its output and the
            // pointer over the panes.
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::Redraw { id: 1 },
            )),
            AppEvent::Terminal(TerminalIntent::PaneGridCursorMoved {
                tab_id: 1,
                position: iced::Point::ORIGIN,
            }),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(is_allowed(dialog, event), "{dialog:?}: {event:?}");
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn a_dialog_keeps_the_window_grips() {
        let grip = AppEvent::ResizeWindow(iced::window::Direction::East);

        for (dialog, _) in dialogs_with_answers() {
            assert!(is_allowed(dialog, &grip), "{dialog:?}");
        }
    }

    #[test]
    fn nothing_is_on_top_of_a_plain_window() {
        let app = app();

        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn each_open_overlay_is_found() {
        for (open, overlay) in OVERLAYS {
            let mut app = app();
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
        }
        for (open, overlay) in CONTAINERS_OVERLAYS {
            let mut app = containers_app_with_lists();
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
        }
    }

    #[test]
    fn the_modes_menu_lies_over_a_header_menu() {
        let mut app = app();
        open_header_menu(&mut app);
        open_modes_menu(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::ModesMenu));
    }

    /// The list lies over the All modes menu and the header menus, and
    /// counts in Containers mode too.
    #[test]
    fn the_notification_list_lies_over_the_other_menus() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_header_menu(&mut app);
        open_modes_menu(&mut app);

        open_notification_list(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::NotificationsList));
    }

    /// The palette lies over the All modes menu and the header menus, and
    /// under the notifications list, in either mode.
    #[test]
    fn the_palette_lies_between_the_menus_and_the_notification_list() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = app.modes.reduce(ModesIntent::Select(mode), ());
            open_header_menu(&mut app);
            open_modes_menu(&mut app);
            open_palette(&mut app);
            let over_the_menus = top_overlay(&app);

            open_notification_list(&mut app);

            assert_eq!(over_the_menus, Some(Overlay::Palette), "{mode:?}");
            assert_eq!(
                top_overlay(&app),
                Some(Overlay::NotificationsList),
                "{mode:?}"
            );
        }
    }

    /// The modes menu belongs to both modes.
    #[test]
    fn the_modes_menu_is_found_in_containers_mode() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_modes_menu(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::ModesMenu));
    }

    /// The pane menu belongs to Terminal mode: it lies under the header
    /// menus, and in Containers mode it is not on screen.
    #[test]
    fn the_pane_menu_lies_under_the_header_menus_in_terminal_mode() {
        let mut app = app();
        open_pane_menu(&mut app);
        let alone = top_overlay(&app);
        open_header_menu(&mut app);
        let under_a_header_menu = top_overlay(&app);
        let _ = app.sidebar.reduce(SidebarIntent::HeaderMenuDismiss, ());
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        assert_eq!(alone, Some(Overlay::TerminalContextMenu));
        assert_eq!(under_a_header_menu, Some(Overlay::HeaderMenu));
        assert_eq!(top_overlay(&app), None);
    }

    /// Esc closes the pane menu, as it closes the other menus.
    #[test]
    fn esc_under_the_pane_menu_closes_it() {
        assert!(matches!(
            Overlay::TerminalContextMenu.escape_event(),
            AppEvent::Terminal(TerminalIntent::CloseAllContextMenus)
        ));
    }

    /// What the terminals report passes under a menu; a press on a pane
    /// does nothing, a right press on a pane opens its menu there, and
    /// another terminal press closes the menus.
    #[test]
    fn the_pane_menu_takes_its_items_and_ignores_a_pane_press() {
        let (_, pane) = iced::widget::pane_grid::State::new(());
        let report = AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::Redraw { id: 2 },
        ));
        let pane_press =
            AppEvent::Terminal(TerminalIntent::PaneClicked { tab_id: 1, pane });
        let right_press = AppEvent::Terminal(TerminalIntent::OpenContextMenu {
            tab_id: 1,
            pane,
            terminal_id: 2,
        });
        let resized = AppEvent::Terminal(TerminalIntent::PaneResized {
            tab_id: 1,
            event: iced::widget::pane_grid::ResizeEvent {
                split: split_of_two_panes(),
                ratio: 0.5,
            },
        });
        let under_pane_menu = |event: &AppEvent| {
            context_menu_guard(Overlay::TerminalContextMenu, event)
        };

        assert_eq!(under_pane_menu(&report), MenuGuard::Allow);
        assert_eq!(under_pane_menu(&pane_press), MenuGuard::Ignore);
        assert_eq!(under_pane_menu(&right_press), MenuGuard::Allow);
        assert_eq!(under_pane_menu(&resized), MenuGuard::Dismiss);
    }

    /// Owner: under the pane menu, a press on the fold arrow of a block or
    /// on the button of the sticky header runs it (the terminal feature
    /// then closes the menu), instead of only closing the menu.
    #[test]
    fn a_fold_arrow_or_header_button_press_under_the_pane_menu_runs() {
        let presses = [
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 2,
                command: otty_ui_term::BlockCommand::ToggleFold(BlockKey::new(
                    3,
                )),
            },
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 2,
                command: otty_ui_term::BlockCommand::ScrollTo(BlockKey::new(3)),
            },
        ];

        for intent in presses {
            let guard = context_menu_guard(
                Overlay::HeaderMenu,
                &AppEvent::Terminal(intent),
            );

            assert_eq!(guard, MenuGuard::Allow);
        }
    }

    /// A split of a pane grid, for an event that names one.
    fn split_of_two_panes() -> iced::widget::pane_grid::Split {
        let (mut panes, pane) = iced::widget::pane_grid::State::new(());
        let (_, split) = panes
            .split(iced::widget::pane_grid::Axis::Vertical, pane, ())
            .expect("the pane splits");

        split
    }

    /// Scenario 2 of the old app: text typed into a terminal under an open
    /// menu or dialog belongs to the overlay, even if the terminal still had
    /// keyboard focus. The inline edit is no layer over the terminals.
    #[test]
    fn terminal_input_is_kept_out_under_every_menu_and_dialog_but_not_an_inline_edit()
     {
        let typed = AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::Write {
                id: 1,
                data: b"a".to_vec(),
            },
        ));

        for (open, overlay) in OVERLAYS {
            let mut app = app();
            open(&mut app);

            assert_eq!(
                is_terminal_input_kept_out(top_overlay(&app), &typed),
                overlay != Overlay::InlineEdit,
                "{overlay:?}"
            );
        }
        for (open, overlay) in CONTAINERS_OVERLAYS {
            let mut app = containers_app_with_lists();
            open(&mut app);

            assert!(
                is_terminal_input_kept_out(top_overlay(&app), &typed),
                "{overlay:?}"
            );
        }
    }

    #[test]
    fn terminal_input_passes_with_nothing_open_and_a_report_always_passes() {
        let typed = AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::Write {
                id: 1,
                data: b"a".to_vec(),
            },
        ));
        let report = AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::Redraw { id: 1 },
        ));

        assert!(!is_terminal_input_kept_out(None, &typed));
        assert!(!is_terminal_input_kept_out(
            Some(Overlay::Settings),
            &report
        ));
    }

    /// The splits can run once the tab in front has a focused pane.
    #[test]
    fn the_command_context_reads_the_focused_pane() {
        let mut app = app();
        let without = command_ctx(&app).has_focused_pane;

        open_terminal_tab(&mut app);

        assert!(!without);
        assert!(command_ctx(&app).has_focused_pane);
    }

    #[test]
    fn the_command_context_reads_the_mode() {
        let mut app = app();
        let in_terminal = command_ctx(&app);
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let in_containers = command_ctx(&app);

        assert_eq!(
            in_terminal,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
                has_local_folder: false,
            }
        );
        assert!(!in_containers.is_terminal_mode);
    }

    /// Close session can run once a tab is active.
    #[test]
    fn the_command_context_reads_the_active_tab() {
        let mut app = app();
        let without = command_ctx(&app).has_active_tab;

        open_terminal_tab(&mut app);

        assert!(!without);
        assert!(command_ctx(&app).has_active_tab);
    }

    /// A press on a Sessions row or the New session button under an open
    /// menu closes the menus and runs; the cross of a row only closes them.
    #[test]
    fn a_tabs_press_under_a_menu_closes_the_menus_and_goes_on() {
        let cross = AppEvent::Tabs(TabsIntent::CloseTab { tab_id: 1 });

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &cross),
            MenuGuard::Dismiss
        );
        for intent in [
            TabsIntent::ActivateTab { tab_id: 1 },
            TabsIntent::OpenTerminalTab { folder: None },
        ] {
            let guard = context_menu_guard(
                Overlay::HeaderMenu,
                &AppEvent::Tabs(intent),
            );

            assert_eq!(guard, MenuGuard::DismissAndRun);
        }
    }

    /// The title of the Sessions section is a sidebar press: under a menu
    /// it only closes the menus.
    #[test]
    fn the_sessions_title_under_a_menu_only_closes_the_menus() {
        let guard = context_menu_guard(
            Overlay::HeaderMenu,
            &AppEvent::Sidebar(SidebarIntent::ToggleSessions),
        );

        assert_eq!(guard, MenuGuard::Dismiss);
    }

    /// Issue 37: under the menu of a plugin, the "⋯" of another plugin
    /// opens its menu there, a press on another plugin only closes the
    /// menus, and Esc closes the menu.
    #[test]
    fn under_the_menu_of_a_plugin_another_plugin_opens_its_menu() {
        let id = || String::from("example");
        let under = |intent| {
            context_menu_guard(Overlay::PluginsMenu, &AppEvent::Plugins(intent))
        };

        assert_eq!(under(PluginsIntent::OpenMenu(id())), MenuGuard::Allow);
        assert_eq!(under(PluginsIntent::Select(id())), MenuGuard::Dismiss);
        assert_eq!(Overlay::PluginsMenu.kind(), Some(OverlayKind::Menu));
        assert!(matches!(
            Overlay::PluginsMenu.escape_event(),
            AppEvent::Plugins(PluginsIntent::CloseMenu)
        ));
    }

    /// A choice in a header menu belongs to another feature, which does
    /// not close the header menu: the frame closes it and runs the choice.
    #[test]
    fn a_choice_in_a_header_menu_closes_it_and_runs() {
        let mut app = app();
        open_header_menu(&mut app);

        let _ = crate::update::update_input(
            &mut app,
            AppEvent::from_menu(AppEvent::QuickLaunch(
                QuickLaunchIntent::HeaderCreateFolder,
            )),
        );

        assert!(!app.sidebar.has_header_menu_open());
        assert!(app.quick_launch.has_inline_edit(), "New folder runs");
    }

    /// The name field and the pointer over the tree pass under a menu; a
    /// right press on a row opens the menu of the Launches tree there, and
    /// other presses in the tree do nothing.
    #[test]
    fn the_launch_menu_takes_its_items_and_ignores_other_tree_presses() {
        let passing = [
            QuickLaunchIntent::InlineEditChanged(String::from("notes")),
            QuickLaunchIntent::InlineEditSubmit,
            QuickLaunchIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            },
        ];
        let ignored = [
            QuickLaunchIntent::BackgroundPressed,
            QuickLaunchIntent::NodePressed {
                path: vec![String::from("Demo")],
            },
        ];
        let right_press =
            AppEvent::QuickLaunch(QuickLaunchIntent::NodeRightClicked {
                path: vec![String::from("Demo")],
            });

        for intent in passing {
            let event = AppEvent::QuickLaunch(intent);
            let guard = context_menu_guard(Overlay::LaunchContextMenu, &event);
            assert_eq!(guard, MenuGuard::Allow);
        }
        for intent in ignored {
            let event = AppEvent::QuickLaunch(intent);
            let guard = context_menu_guard(Overlay::LaunchContextMenu, &event);
            assert_eq!(guard, MenuGuard::Ignore);
        }
        assert_eq!(
            context_menu_guard(Overlay::LaunchContextMenu, &right_press),
            MenuGuard::Allow
        );
        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &right_press),
            MenuGuard::DismissAndRun
        );
    }

    /// A key that is no shortcut, pointer moves and terminal events keep an
    /// inline edit; a press anywhere else and a command cancel it.
    #[test]
    fn only_typing_pointer_moves_and_terminal_events_keep_an_inline_edit() {
        let keeping = [
            AppEvent::Keyboard(key_press(
                Physical::Code(Code::KeyA),
                "a",
                Modifiers::empty(),
            )),
            AppEvent::QuickLaunch(QuickLaunchIntent::InlineEditChanged(
                String::from("notes"),
            )),
            AppEvent::QuickLaunch(QuickLaunchIntent::InlineEditSubmit),
            AppEvent::QuickLaunch(QuickLaunchIntent::NodeHovered {
                path: vec![String::from("Demo")],
            }),
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
            AppEvent::Explorer(ExplorerIntent::NodeHovered { path: None }),
            AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar),
            AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved {
                position: iced::Point::ORIGIN,
            }),
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::Redraw { id: 1 },
            )),
        ];
        let cancelling = [
            AppEvent::Command(AppCommand::ToggleRightPanel),
            AppEvent::TitleBar(TitleBarIntent::PressTitleBar),
            AppEvent::Sidebar(SidebarIntent::ToggleRight),
            AppEvent::Explorer(ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            }),
            AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundPressed),
        ];

        for event in &keeping {
            assert!(!inline_edit_guard(event), "{event:?}");
        }
        for event in &cancelling {
            assert!(inline_edit_guard(event), "{event:?}");
        }
    }

    #[test]
    fn esc_sends_the_close_intent_of_each_overlay() {
        assert!(matches!(
            Overlay::Settings.escape_event(),
            AppEvent::Settings(SettingsIntent::Close)
        ));
        assert!(matches!(
            Overlay::ModesMenu.escape_event(),
            AppEvent::Modes(ModesIntent::CloseMenu)
        ));
        assert!(matches!(
            Overlay::HeaderMenu.escape_event(),
            AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss)
        ));
        assert!(matches!(
            Overlay::NotificationsList.escape_event(),
            AppEvent::Notifications(NotificationsIntent::CloseList)
        ));
        assert!(matches!(
            Overlay::Palette.escape_event(),
            AppEvent::Palette(PaletteIntent::Dismiss)
        ));
        assert!(matches!(
            Overlay::SecretPrompt.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchIntent::SecretPromptCancel)
        ));
        assert!(matches!(
            Overlay::LaunchContextMenu.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchIntent::ContextMenuDismiss)
        ));
        assert!(matches!(
            Overlay::InlineEdit.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchIntent::CancelInlineEdit)
        ));
        assert!(matches!(
            Overlay::ContainersCreate.escape_event(),
            AppEvent::Containers(ContainersIntent::Create(
                CreateIntent::Cancelled
            ))
        ));
        assert!(matches!(
            Overlay::ContainersConfirm.escape_event(),
            AppEvent::Containers(ContainersIntent::Actions(
                ActionsIntent::ConfirmCancelled
            ))
        ));
        assert!(matches!(
            Overlay::ContainersRowMenu.escape_event(),
            AppEvent::Containers(ContainersIntent::Actions(
                ActionsIntent::CloseRowMenu
            ))
        ));
    }

    /// Owner, 2026-11-23: under the menu of an Agents session, a right
    /// press on another row opens that row's menu with the one press.
    #[test]
    fn under_an_agents_menu_another_row_opens_its_menu() {
        let other_row = AppEvent::Agents(AgentsIntent::OpenSessionMenu(1));

        assert_eq!(
            context_menu_guard(Overlay::AgentsMenu, &other_row),
            MenuGuard::Allow
        );
        assert_eq!(
            context_menu_guard(Overlay::PluginsMenu, &other_row),
            MenuGuard::DismissAndRun
        );
    }

    /// Where the menus of Agents mode open in the tests, inside a main area
    /// big enough for each of them.
    fn agents_menu_ctx() -> AgentsCtx {
        AgentsCtx {
            cursor: iced::Point::new(40.0, 40.0),
            screen_size: iced::Size::new(1000.0, 700.0),
            has_local_pane: false,
            local_folder: None,
        }
    }

    /// An app in Agents mode with the default Codex, logged in, with
    /// one session in `/work/otty`, and the added Codex · Work, not
    /// logged in; the project `/work/otty` is selected. So every item of
    /// every menu of the mode can be pressed.
    fn agents_app_with_a_session() -> App {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Agents), ());
        let connected = otty_agents::ProfileStatus::Connected;
        for intent in [
            profiles_loaded(vec![
                (codex_profile(), connected),
                (work_profile(), crate::testing::agents::not_logged_in()),
            ]),
            one_session_loaded(codex_profile(), "c1"),
            AgentsIntent::SelectProject(std::path::PathBuf::from(PROJECT)),
        ] {
            let _ = app.agents.reduce(intent, agents_menu_ctx());
        }

        app
    }

    /// Open a menu of Agents mode with `intent`.
    fn open_agents_menu_with(app: &mut App, intent: AgentsIntent) {
        let _ = app.agents.reduce(intent, agents_menu_ctx());
        assert_eq!(top_overlay(app), Some(Overlay::AgentsMenu));
    }

    /// What a press on `label` in the open menu of Agents mode sends. Only
    /// the menu layer is drawn, so the label can not be found in the left
    /// panel or the main area under it.
    fn pressed_in_agents_menu(app: &App, label: &str) -> Vec<AppEvent> {
        let theme = otty_app_theme::theme::AppTheme::default();
        let layer = iced::widget::Column::with_children(
            crate::screens::agents::overlays(
                app,
                otty_app_theme::theme::ThemeProps::new(&theme),
            ),
        );
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 700.0),
            layer,
        );

        ui.click(label)
            .unwrap_or_else(|_| panic!("the item {label} should show"));

        ui.into_messages().collect()
    }

    /// Press each of `labels` in the open menu of Agents mode and check
    /// that each press sends an item marked as sent from the menu, which
    /// the frame lets through to it with no list of its items.
    fn assert_items_pass(app: &App, labels: &[&str]) {
        for label in labels {
            let sent = pressed_in_agents_menu(app, label);

            assert!(!sent.is_empty(), "{label} sends an item");
            for event in &sent {
                assert!(
                    matches!(event, AppEvent::FromMenu(_)),
                    "{label}: {event:?}"
                );
            }
        }
    }

    /// Owner, issue 26: Edit and Remove of "⋯" did nothing, the frame took
    /// them for a press outside the menu. Every item of the profile menu
    /// now goes to it.
    #[test]
    fn every_item_of_the_agents_profile_menu_passes() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(
            &mut app,
            AgentsIntent::OpenProfileMenu(work_profile()),
        );

        assert_items_pass(
            &app,
            &[
                i18n::t(i18n::Key::AgentsRename),
                i18n::t(i18n::Key::AgentsOpenFolder),
                i18n::t(i18n::Key::Edit),
                i18n::t(i18n::Key::AgentsLogIn),
                i18n::t(i18n::Key::AgentsHide),
                i18n::t(i18n::Key::ButtonRemove),
            ],
        );
    }

    /// Spec 2026-11-19, Decision 7: every item of the menu of an account
    /// goes to it.
    #[test]
    fn every_item_of_the_agents_account_menu_passes() {
        let mut app = agents_app_with_a_session();
        let account = otty_agents::Account::new(
            "openai",
            otty_agents::ProviderId::new("openai"),
            "chatgpt:made-up-account",
            "you@example.com",
            otty_agents::AccountKind::Subscription,
            Vec::new(),
        );
        open_agents_menu_with(&mut app, AgentsIntent::OpenAccountMenu(account));

        assert_items_pass(
            &app,
            &[
                i18n::t(i18n::Key::AgentsRename),
                i18n::t(i18n::Key::AgentsOpenUsagePage),
                i18n::t(i18n::Key::AgentsHide),
            ],
        );
    }

    /// The "⋯" of All accounts and of All profiles: their items go to
    /// them.
    #[test]
    fn every_item_of_the_agents_overview_menus_passes() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllAccountsMenu);
        assert_items_pass(&app, &[i18n::t(i18n::Key::AgentsShowFoundReport)]);

        let _ = app
            .agents
            .reduce(AgentsIntent::ShowHiddenProfiles(true), agents_menu_ctx());
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllProfilesMenu);
        assert_items_pass(
            &app,
            &[i18n::t(i18n::Key::AgentsHideHiddenProfiles)],
        );
        let _ = app
            .agents
            .reduce(AgentsIntent::ShowHiddenAccounts(true), agents_menu_ctx());
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllAccountsMenu);
        assert_items_pass(
            &app,
            &[i18n::t(i18n::Key::AgentsHideHiddenAccounts)],
        );
    }

    #[test]
    fn every_item_of_the_agents_session_menu_passes() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenSessionMenu(0));

        assert_items_pass(
            &app,
            &[
                i18n::t(i18n::Key::AgentsContinue),
                i18n::t(i18n::Key::AgentsContinueInSplit),
                i18n::t(i18n::Key::AgentsCopyResumeCommand),
                i18n::t(i18n::Key::AgentsCopySessionId),
                i18n::t(i18n::Key::AgentsShowStatistics),
            ],
        );
    }

    #[test]
    fn every_item_of_the_agents_new_session_menu_passes() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenNewSessionMenu);

        assert_items_pass(&app, &["Codex", "Codex · Work"]);
    }

    /// A choice that is on lets Clear all filters be pressed too.
    #[test]
    fn every_item_of_the_agents_funnel_menu_passes() {
        let mut app = agents_app_with_a_session();
        let _ = app.agents.reduce(
            AgentsIntent::ProjectFilter(
                otty_app_agents::ProjectFilterChange::RecentOnly(true),
            ),
            agents_menu_ctx(),
        );
        open_agents_menu_with(&mut app, AgentsIntent::OpenFilterMenu);

        assert_items_pass(
            &app,
            &[
                i18n::t(i18n::Key::AgentsFilterAllAgents),
                "Codex",
                "Codex",
                i18n::t(i18n::Key::AgentsFilterRecent),
                i18n::t(i18n::Key::AgentsFilterMissing),
                i18n::t(i18n::Key::AgentsFilterSortByName),
                i18n::t(i18n::Key::AgentsFilterClearAll),
            ],
        );
    }

    /// Owner, issue 26: Edit and Remove of the profile menu reach the
    /// feature through the frame and open their dialogs.
    #[test]
    fn edit_and_remove_of_the_profile_menu_open_their_dialogs() {
        let mut app = agents_app_with_a_session();
        let open_menu = AgentsIntent::OpenProfileMenu(work_profile());
        let _ = app.agents.reduce(open_menu.clone(), agents_menu_ctx());

        let _ = crate::update::update_input(
            &mut app,
            AppEvent::from_menu(AppEvent::Agents(AgentsIntent::EditProfile(
                work_profile(),
            ))),
        );
        let edits = app.agents.top_overlay()
            == Some(otty_app_agents::AgentsOverlay::ProfileDialog);
        let _ = app.agents.reduce(
            AgentsIntent::Dialog(ProfileDialogIntent::Cancelled),
            agents_menu_ctx(),
        );
        let _ = app.agents.reduce(open_menu, agents_menu_ctx());
        let _ = crate::update::update_input(
            &mut app,
            AppEvent::from_menu(AppEvent::Agents(AgentsIntent::RemoveProfile(
                work_profile(),
            ))),
        );

        assert!(edits, "Edit opens the profile dialog");
        assert_eq!(
            app.agents.top_overlay(),
            Some(otty_app_agents::AgentsOverlay::RemoveDialog),
            "Remove asks first"
        );
    }

    /// Owner, 2026-11-23: Today, 7 days and 30 days of the usage block of
    /// a profile page did nothing, the frame took them for a press outside
    /// the menu. Every item of the menu now goes to it.
    #[test]
    fn every_item_of_the_agents_period_menu_passes() {
        let mut app = agents_app_with_a_session();

        open_agents_menu_with(&mut app, AgentsIntent::OpenPeriodMenu);
        assert_items_pass(
            &app,
            &[
                i18n::t(i18n::Key::AgentsToday),
                i18n::t(i18n::Key::AgentsSevenDays),
                i18n::t(i18n::Key::AgentsThirtyDays),
            ],
        );
    }

    /// The pointer moving over the rows of a usage block is no press: a
    /// menu stays open, as it does over the Files tree.
    #[test]
    fn the_pointer_over_the_usage_rows_keeps_a_menu_open() {
        for event in [
            AgentsIntent::HoverUsageRow(Some(String::from("gpt-5"))),
            AgentsIntent::HoverUsageRow(None),
        ] {
            assert_eq!(
                context_menu_guard(
                    Overlay::HeaderMenu,
                    &AppEvent::Agents(event)
                ),
                MenuGuard::Allow
            );
        }
    }

    /// Codex, its 400 000 tokens of gpt-5 counted today at 09:00 and
    /// priced by the Codex part at the made-up list price of $1.25 per
    /// million input tokens, the prices read, and its page shown.
    fn agents_app_with_a_counted_profile() -> App {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Agents), ());
        let api_key = otty_agents::ProfileStatus::Connected;
        let tokens = otty_agents::ModelTokens::new(
            Some("gpt-5"),
            otty_agents::TokenCount::new(400_000, 0, 0),
            1,
        )
        .with_money(otty_agents::MoneyTotal::at_api_prices(
            otty_agents::Dollars::from_micro_dollars(500_000),
            1,
        ));
        let parts = vec![otty_agents::ProjectModelTokens::new(
            Some(std::path::PathBuf::from(PROJECT)),
            tokens.clone(),
        )];
        let hours = (0..24)
            .map(|hour| match hour {
                9 => otty_agents::HourSpend::new(hour, parts.clone()),
                _ => otty_agents::HourSpend::new(hour, Vec::new()),
            })
            .collect();
        let today = otty_agents::DaySpend::new(
            jiff::civil::date(2026, 11, 16),
            vec![tokens],
        )
        .with_parts(parts);
        let spend =
            otty_agents::Spend::new(vec![today], Vec::new()).with_hours(hours);
        let gpt_5 = otty_agents::TokenPrices {
            input: otty_agents::Dollars::from_micro_dollars(1_250_000),
            ..otty_agents::TokenPrices::default()
        };
        let prices = otty_agents::FetchedPrices::new(
            otty_agents::PriceTable::new(vec![otty_agents::ModelPrice::new(
                "gpt-5", gpt_5,
            )]),
            std::time::SystemTime::UNIX_EPOCH,
        );

        for intent in [
            profiles_loaded(vec![(codex_profile(), api_key)]),
            AgentsIntent::SpendLoaded(otty_app_agents::LoadedSpend {
                profile: codex_profile(),
                spend: otty_app_agents::SpendCount::Counted(spend),
                counted_at: std::time::SystemTime::UNIX_EPOCH,
            }),
            AgentsIntent::PricesLoaded(otty_app_agents::LoadedPrices {
                prices: otty_agents::fixtures::walked(std::iter::once((
                    otty_agents::SourceId::LiteLlmPrices,
                    Ok(prices),
                ))),
                read_at: std::time::SystemTime::UNIX_EPOCH,
            }),
            AgentsIntent::SelectProfile(codex_profile()),
        ] {
            let _ = app.agents.reduce(intent, agents_menu_ctx());
        }

        app
    }

    /// Open the menu of the usage block with `open` and press `label` in
    /// it, through the frame as the user does.
    fn pick_in_usage_menu(app: &mut App, open: AgentsIntent, label: &str) {
        open_agents_menu_with(app, open);

        for event in pressed_in_agents_menu(app, label) {
            let _ = crate::update::update_input(app, event);
        }
    }

    /// Whether the main area of Agents mode shows `text`.
    fn main_area_shows(app: &App, text: &str) -> bool {
        let theme = otty_app_theme::theme::AppTheme::default();
        let main = crate::screens::agents::main(
            app,
            otty_app_theme::theme::ThemeProps::new(&theme),
        );
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 700.0),
            main,
        );

        ui.find(text).is_ok()
    }

    /// Owner, 2026-11-23: a period picked in its menu shows on the
    /// profile page: the menu closes and the total under Usage follows.
    /// Issue 14 of spec 2026-11-23: the total says the tokens and their
    /// money at API prices, with no switch between them.
    #[test]
    fn a_period_picked_in_its_menu_shows_on_the_page() {
        let mut app = agents_app_with_a_counted_profile();
        let both = i18n::filled(
            i18n::Key::TplAgentsTokensAndMoney,
            &[("tokens", "400K"), ("money", "~$0.50")],
        );
        let total = |amount: &str, key: i18n::Key| {
            i18n::filled(key, &[("amount", amount), ("count", "7")])
        };
        let week = main_area_shows(
            &app,
            &total(&both, i18n::Key::TplAgentsAmountInDays),
        );

        pick_in_usage_menu(
            &mut app,
            AgentsIntent::OpenPeriodMenu,
            i18n::t(i18n::Key::AgentsToday),
        );
        let today = main_area_shows(
            &app,
            &total(&both, i18n::Key::TplAgentsAmountToday),
        );
        let is_closed = top_overlay(&app).is_none();

        assert!(week, "the page opens on 7 days of tokens and money");
        assert!(today, "Today shows the tokens and the money of today");
        assert!(is_closed, "the menu closes");
    }

    /// Issue 42: the pointer moving over a disabled item of an open menu,
    /// whose reason then shows, and a press on it go through the window
    /// and the input checks and keep the menu open.
    #[test]
    fn hovering_and_pressing_a_disabled_item_keeps_the_menu_open() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllProfilesMenu);
        let mut window = RunningApp::new(app);
        let show_hidden = i18n::filled(
            i18n::Key::TplAgentsShowHiddenProfiles,
            &[("count", "0")],
        );
        let off = window
            .find(&show_hidden)
            .expect("Show hidden profiles shows, off with none hidden");

        for point in [
            off.center(),
            iced::Point::new(off.x + 1.0, off.y + 1.0),
            iced::Point::new(off.x + off.width, off.center_y()),
        ] {
            window.move_to(point);
        }
        let hovered = top_overlay(window.app());
        window.click(iced::mouse::Button::Left);

        assert_eq!(hovered, Some(Overlay::AgentsMenu));
        assert_eq!(top_overlay(window.app()), Some(Overlay::AgentsMenu));
    }

    /// Owner, 2026-11-23: with the period menu of a profile page open, one
    /// press on the Tokens drop-down closes it and opens the measure menu,
    /// through the window and the input checks; a press on the drop-down
    /// of the open menu closes it, and a press away closes a menu too.
    #[test]
    fn one_press_on_another_drop_down_switches_the_menu() {
        let mut window = RunningApp::new(agents_app_with_a_counted_profile());
        let thirty_days = i18n::t(i18n::Key::AgentsThirtyDays);
        let money = i18n::t(i18n::Key::AgentsNumberMoney);

        window.click_text(i18n::t(i18n::Key::AgentsSevenDays));
        let period_open = window.find(thirty_days).is_some();
        window.click_text(i18n::t(i18n::Key::AgentsTokens));
        let switched =
            window.find(money).is_some() && window.find(thirty_days).is_none();
        window.click_text(i18n::t(i18n::Key::AgentsTokens));
        let closed = top_overlay(window.app()).is_none();
        window.click_text(i18n::t(i18n::Key::AgentsSevenDays));
        window.move_to(iced::Point::new(1000.0, 740.0));
        window.click(iced::mouse::Button::Left);

        assert!(period_open, "the period menu opens");
        assert!(switched, "one press opens the measure menu in its place");
        assert!(closed, "the drop-down of the open menu closes it");
        assert_eq!(top_overlay(window.app()), None, "a press away closes");
    }

    /// Owner, 2026-11-23: the open-menu guard keeps no list of the items
    /// of a menu. A new item of a menu, sending what no menu sent before,
    /// reaches its feature with no change to the guard.
    #[test]
    fn a_new_item_of_a_menu_works_without_touching_the_guard() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllProfilesMenu);
        let palette = otty_ui_kit::palette::UiPalette::otty_dark();
        let menu = otty_ui_kit::components::menu::Menu::new(&palette).item(
            otty_ui_kit::components::menu::MenuItem::new("A new item")
                .on_press(AgentsIntent::OpenAddProfile),
        );
        let layer = crate::screens::overlay_layer(
            otty_app_slots::Overlay::menu(
                otty_ui_kit::components::context_menu::context_menu(
                    menu,
                    agents_menu_ctx_place(),
                ),
            ),
            AppEvent::Agents,
        );
        let mut ui = iced_test::simulator(layer);

        ui.click("A new item").expect("the new item shows");
        for event in ui.into_messages().collect::<Vec<_>>() {
            let _ = crate::update::update_input(&mut app, event);
        }

        assert_eq!(
            app.agents.top_overlay(),
            Some(otty_app_agents::AgentsOverlay::ProfileDialog)
        );
    }

    /// Where the menus of Agents mode open in the tests.
    fn agents_menu_ctx_place()
    -> otty_ui_kit::components::context_menu::MenuPlace {
        let ctx = agents_menu_ctx();

        otty_ui_kit::components::context_menu::MenuPlace::new(
            ctx.cursor,
            ctx.screen_size,
        )
    }

    /// A press no widget took closes the open menu, and is no answer to a
    /// dialog.
    #[test]
    fn a_press_away_closes_the_menus() {
        let mut app = agents_app_with_a_session();
        open_agents_menu_with(&mut app, AgentsIntent::OpenAllProfilesMenu);

        let guard =
            context_menu_guard(Overlay::AgentsMenu, &AppEvent::PressedAway);
        let _ = crate::update::update_input(&mut app, AppEvent::PressedAway);

        assert_eq!(guard, MenuGuard::Dismiss);
        assert_eq!(top_overlay(&app), None);
        assert_eq!(
            dialog_guard(Overlay::AgentsProfileDialog, &AppEvent::PressedAway),
            MenuGuard::Ignore
        );
    }

    /// The cross of a toast under a menu is a click outside the menu: it
    /// only closes the menu.
    #[test]
    fn a_toast_cross_under_a_menu_only_closes_the_menus() {
        let event =
            AppEvent::Notifications(NotificationsIntent::DismissToast(1));

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &event),
            MenuGuard::Dismiss
        );
    }

    #[test]
    fn choosing_a_mode_or_opening_settings_closes_the_menus_and_goes_on() {
        let events = [
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
            AppEvent::TitleBar(TitleBarIntent::OpenSettings),
        ];

        for event in events {
            assert_eq!(
                context_menu_guard(Overlay::HeaderMenu, &event),
                MenuGuard::DismissAndRun
            );
        }
    }

    #[test]
    fn a_panel_button_only_closes_the_menus() {
        let presses = [
            TitleBarIntent::ToggleLeftPanel,
            TitleBarIntent::ToggleRightPanel,
            TitleBarIntent::ToggleFocusMode,
        ];

        for press in presses {
            let event = AppEvent::TitleBar(press);

            assert_eq!(
                context_menu_guard(Overlay::HeaderMenu, &event),
                MenuGuard::Dismiss
            );
        }
    }

    /// Owner, 2026-11-23: the bell and the palette button open their menu
    /// with the one press that closes another menu; under their own menu
    /// they close it.
    #[test]
    fn the_bell_and_the_palette_button_close_another_menu_and_open() {
        let bell = AppEvent::TitleBar(TitleBarIntent::OpenNotifications);
        let palette = AppEvent::TitleBar(TitleBarIntent::OpenCommandPalette);

        assert_eq!(
            context_menu_guard(Overlay::AgentsMenu, &bell),
            MenuGuard::DismissAndRun
        );
        assert_eq!(
            context_menu_guard(Overlay::AgentsMenu, &palette),
            MenuGuard::DismissAndRun
        );
        assert_eq!(
            context_menu_guard(Overlay::NotificationsList, &bell),
            MenuGuard::Dismiss
        );
        assert_eq!(
            context_menu_guard(Overlay::Palette, &palette),
            MenuGuard::Dismiss
        );
    }

    #[test]
    fn a_window_button_leaves_the_menus_open() {
        for press in [
            TitleBarIntent::MinimizeWindow,
            TitleBarIntent::ToggleMaximize,
            TitleBarIntent::CloseWindow,
        ] {
            let event = AppEvent::TitleBar(press);

            assert_eq!(
                context_menu_guard(Overlay::HeaderMenu, &event),
                MenuGuard::Allow
            );
        }
    }

    #[test]
    fn moving_over_the_title_bar_keeps_the_menus() {
        let event = AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar);

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &event),
            MenuGuard::Allow
        );
    }

    /// The chevron of a section header opens its menu with one press under
    /// another menu, and goes to the sidebar under a header menu.
    #[test]
    fn the_chevron_of_a_header_opens_its_menu_under_another_menu() {
        let chevron = AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
            HeaderMenu::Sessions,
        ));

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &chevron),
            MenuGuard::Allow
        );
        assert_eq!(
            context_menu_guard(Overlay::ModesMenu, &chevron),
            MenuGuard::DismissAndRun
        );
    }

    /// The title of the always open Installed section sends the dismiss of
    /// the header menu: from outside the menu it closes the menus.
    #[test]
    fn the_header_menu_dismiss_from_outside_closes_the_menus() {
        let guard = context_menu_guard(
            Overlay::HeaderMenu,
            &AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss),
        );

        assert_eq!(guard, MenuGuard::Dismiss);
    }

    #[test]
    fn the_right_panel_toggle_closes_the_menus() {
        let toggle = AppEvent::Sidebar(SidebarIntent::ToggleRight);

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &toggle),
            MenuGuard::Dismiss
        );
    }

    #[test]
    fn hovering_a_files_row_keeps_the_menus() {
        // A row sends the path when the pointer enters it, and no path when
        // the pointer leaves it or the whole tree.
        let paths = [Some(vec![String::from("src")]), None];

        for path in paths {
            let event =
                AppEvent::Explorer(ExplorerIntent::NodeHovered { path });

            assert_eq!(
                context_menu_guard(Overlay::HeaderMenu, &event),
                MenuGuard::Allow
            );
        }
    }

    #[test]
    fn pressing_a_files_row_still_closes_the_menus() {
        let event = AppEvent::Explorer(ExplorerIntent::NodePressed {
            path: vec![String::from("src")],
        });

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &event),
            MenuGuard::Dismiss
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn a_window_grip_leaves_the_menus_open() {
        let grip = AppEvent::ResizeWindow(iced::window::Direction::East);

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &grip),
            MenuGuard::Allow
        );
    }

    #[test]
    fn a_command_under_an_open_menu_closes_the_menus_and_goes_on() {
        let event = AppEvent::Command(AppCommand::ToggleFocusMode);

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &event),
            MenuGuard::DismissAndRun
        );
    }

    #[test]
    fn a_key_that_is_not_a_shortcut_does_nothing_under_a_menu() {
        let event = AppEvent::Keyboard(key_press(
            Physical::Code(Code::KeyA),
            "a",
            Modifiers::empty(),
        ));

        assert_eq!(
            context_menu_guard(Overlay::HeaderMenu, &event),
            MenuGuard::Ignore
        );
    }

    /// A press in the engine tree, the lists, the inspector tabs or on Prune
    /// closes an open menu and cancels an inline edit, as any other press in
    /// the window does.
    #[test]
    fn a_click_in_the_containers_views_closes_menus_and_inline_edits() {
        use otty_app_containers::InspectorTab;

        let clicks = [
            ContainersIntent::ToggleEngine(
                otty_containers::EngineAddress::Tcp(String::from("h:1")),
            ),
            ContainersIntent::Refresh,
            ContainersIntent::SelectRow(0),
            ContainersIntent::SelectInspectorTab(InspectorTab::Logs),
            ContainersIntent::Actions(ActionsIntent::Prune),
        ]
        .map(AppEvent::Containers);

        for event in clicks {
            assert!(matches!(
                context_menu_guard(Overlay::HeaderMenu, &event),
                MenuGuard::Dismiss
            ));
            assert!(inline_edit_guard(&event));
        }
    }

    /// The Files menu is a menu, its delete question a dialog, and its name
    /// field neither, like the name field of the Launches tree.
    #[test]
    fn each_files_overlay_is_found_on_top_with_its_kind() {
        let openers: [(Opener, Overlay, Option<OverlayKind>); 3] = [
            (
                open_files_menu,
                Overlay::FilesContextMenu,
                Some(OverlayKind::Menu),
            ),
            (
                open_files_delete_dialog,
                Overlay::FilesDeleteDialog,
                Some(OverlayKind::Dialog),
            ),
            (start_files_inline_edit, Overlay::FilesInlineEdit, None),
        ];

        for (open, overlay, kind) in openers {
            let mut app = app();
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
            assert_eq!(overlay.kind(), kind, "{overlay:?}");
        }
    }

    /// Esc closes the Files menu, cancels its delete question and its name
    /// field.
    #[test]
    fn esc_closes_or_cancels_each_files_overlay() {
        let expected = [
            (
                Overlay::FilesContextMenu,
                ExplorerIntent::ContextMenuDismiss,
            ),
            (Overlay::FilesDeleteDialog, ExplorerIntent::DeleteCancelled),
            (Overlay::FilesInlineEdit, ExplorerIntent::CancelInlineEdit),
        ];

        for (overlay, intent) in expected {
            let event = overlay.escape_event();
            let AppEvent::Explorer(sent) = event else {
                panic!("{overlay:?} should close through Files");
            };

            assert_eq!(
                std::mem::discriminant(&sent),
                std::mem::discriminant(&intent),
                "{overlay:?}"
            );
        }
    }

    /// The delete question waits for its answer: only Delete and Cancel
    /// pass, a press on a row or a shortcut does nothing.
    #[test]
    fn the_files_delete_dialog_takes_only_its_answers() {
        let answers = [
            AppEvent::Explorer(ExplorerIntent::DeleteConfirmed),
            AppEvent::Explorer(ExplorerIntent::DeleteCancelled),
            AppEvent::Explorer(ExplorerIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            }),
        ];
        let others = [
            AppEvent::Explorer(ExplorerIntent::NodePressed {
                path: vec![String::from("notes.txt")],
            }),
            AppEvent::Command(AppCommand::ToggleFocusMode),
            AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundPressed),
        ];

        for event in answers {
            let guard = dialog_guard(Overlay::FilesDeleteDialog, &event);
            assert_eq!(guard, MenuGuard::Allow, "{event:?}");
        }
        for event in others {
            let guard = dialog_guard(Overlay::FilesDeleteDialog, &event);
            assert_eq!(guard, MenuGuard::Ignore, "{event:?}");
        }
    }

    /// Under a menu, the name field of Files and the pointer over the tree
    /// pass; a right press on a row opens the Files menu there, and a
    /// press on a Files row only closes the menus.
    #[test]
    fn under_a_menu_the_files_menu_passes_and_a_files_press_closes_it() {
        let passing = [
            ExplorerIntent::InlineEditChanged(String::from("a")),
            ExplorerIntent::InlineEditSubmit,
            ExplorerIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            },
            ExplorerIntent::NodeHovered { path: None },
            ExplorerIntent::NodeRightClicked {
                path: vec![String::from("notes.txt")],
            },
        ];

        for intent in passing {
            let event = AppEvent::Explorer(intent);
            assert_eq!(
                context_menu_guard(Overlay::FilesContextMenu, &event),
                MenuGuard::Allow
            );
        }
        let press = AppEvent::Explorer(ExplorerIntent::NodePressed {
            path: vec![String::from("notes.txt")],
        });
        assert_eq!(
            context_menu_guard(Overlay::FilesContextMenu, &press),
            MenuGuard::Dismiss
        );
    }

    /// Typing in the Files name field and moving the pointer leave it; a
    /// press anywhere else, a press in the Launches tree too, cancels it.
    #[test]
    fn only_typing_and_pointer_moves_leave_the_files_name_field() {
        let leaving = [
            AppEvent::Explorer(ExplorerIntent::InlineEditChanged(
                String::from("a"),
            )),
            AppEvent::Explorer(ExplorerIntent::InlineEditSubmit),
            AppEvent::Explorer(ExplorerIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            }),
            AppEvent::QuickLaunch(QuickLaunchIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            }),
        ];
        let cancelling = [
            AppEvent::Explorer(ExplorerIntent::BackgroundPressed),
            AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundPressed),
            AppEvent::QuickLaunch(QuickLaunchIntent::InlineEditSubmit),
            AppEvent::Command(AppCommand::ToggleFocusMode),
        ];

        for event in leaving {
            assert!(!files_inline_edit_guard(&event), "{event:?}");
        }
        for event in cancelling {
            assert!(files_inline_edit_guard(&event), "{event:?}");
        }
        // Typing in the Files name field cancels a Launches one.
        let typing = AppEvent::Explorer(ExplorerIntent::InlineEditSubmit);
        assert!(inline_edit_guard(&typing));
    }
}
