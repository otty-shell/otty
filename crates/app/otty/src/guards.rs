//! What input from the user does while a menu or a dialog is open, which
//! overlay lies on top of the window, and what decides whether a command can
//! run now. The input checks only see input: what follows from it, and what
//! arrives on its own, skips them (see `update_internal` in `update.rs`), so
//! it can not get lost.
//!
//! The overlays are the SSH secret prompt, the Settings dialog, the
//! notifications list, the command palette, the All modes menu, the header
//! menus, the pane menu of the terminal, the context menu and the name field
//! (the inline edit) of the Launches tree, and the create dialogs, the
//! confirmation dialog and the row menu of Containers mode. A new dialog adds
//! its answers to [`dialog_guard`].

use otty_app_containers::{ActionsIntent, ContainersIntent, CreateIntent};
use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
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
    /// A create dialog of Containers mode, such as Pull image.
    ContainersCreate,
    /// The dialog of Containers mode that asks to confirm a destructive
    /// action.
    ContainersConfirm,
    /// The menu of a row in Containers mode.
    ContainersRowMenu,
}

impl Overlay {
    /// Return whether the overlay is a menu or a dialog. A click outside a
    /// menu closes it, and a shortcut closes it and runs. A dialog waits for
    /// its answer (see [`dialog_guard`]). The inline edit is neither: it is
    /// a field in the tree, not a layer over the terminals, and any other
    /// input cancels it (see [`inline_edit_guard`]).
    pub(crate) fn kind(self) -> Option<OverlayKind> {
        match self {
            Overlay::SecretPrompt
            | Overlay::Settings
            | Overlay::ContainersCreate
            | Overlay::ContainersConfirm => Some(OverlayKind::Dialog),
            Overlay::NotificationsList
            | Overlay::Palette
            | Overlay::ModesMenu
            | Overlay::HeaderMenu
            | Overlay::TerminalContextMenu
            | Overlay::LaunchContextMenu
            | Overlay::ContainersRowMenu => Some(OverlayKind::Menu),
            Overlay::InlineEdit => None,
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
            Overlay::ContainersCreate => AppEvent::Containers(
                ContainersIntent::Create(CreateIntent::Cancelled),
            ),
            Overlay::ContainersConfirm => AppEvent::Containers(
                ContainersIntent::Actions(ActionsIntent::ConfirmCancelled),
            ),
            Overlay::ContainersRowMenu => AppEvent::Containers(
                ContainersIntent::Actions(ActionsIntent::CloseRowMenu),
            ),
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

/// Classify input from the user when a menu is on top.
pub(crate) fn context_menu_guard(event: &AppEvent) -> MenuGuard {
    use MenuGuard::*;

    match event {
        AppEvent::Sidebar(intent) => match intent {
            SidebarIntent::HeaderMenuDismiss
            | SidebarIntent::WorkspaceCursorMoved { .. } => Allow,
            SidebarIntent::HeaderMenuOpen(_) => Ignore,
            _ => Dismiss,
        },
        // The items and the dismiss layer of the Launches context menu, the
        // name field and the pointer over the tree pass; other presses in
        // the Launches section do nothing under a menu.
        AppEvent::QuickLaunch(intent) => match intent {
            QuickLaunchIntent::ContextMenuDismiss
            | QuickLaunchIntent::ContextMenuAction(_)
            | QuickLaunchIntent::InlineEditChanged(_)
            | QuickLaunchIntent::InlineEditSubmit
            | QuickLaunchIntent::CursorMoved { .. } => Allow,
            _ => Ignore,
        },
        // The items of the pane menu and its dismiss layer, and what the
        // terminals report, pass. A press on a pane or its header under a
        // menu does nothing; other terminal input closes the menus.
        AppEvent::Terminal(intent) => match intent {
            TerminalIntent::CloseContextMenu { .. }
            | TerminalIntent::CopySelection { .. }
            | TerminalIntent::PasteIntoPrompt { .. }
            | TerminalIntent::CopySelectedBlockContent { .. }
            | TerminalIntent::CopySelectedBlockCommand { .. }
            | TerminalIntent::SplitPane { .. }
            | TerminalIntent::ClosePane { .. }
            | TerminalIntent::Widget(_)
            | TerminalIntent::PaneGridCursorMoved { .. } => Allow,
            TerminalIntent::OpenContextMenu { .. }
            | TerminalIntent::PaneClicked { .. } => Ignore,
            _ => Dismiss,
        },
        // The items and the dismiss layer of the Containers row menu.
        AppEvent::Containers(ContainersIntent::Actions(
            ActionsIntent::Act(_) | ActionsIntent::CloseRowMenu,
        )) => Allow,
        // The first press on a panel button, the palette button or the bell
        // only closes the menu; opening settings closes it and goes on.
        AppEvent::TitleBar(intent) => match intent {
            TitleBarIntent::ToggleLeftPanel
            | TitleBarIntent::ToggleFilesPanel
            | TitleBarIntent::ToggleFocusMode
            | TitleBarIntent::OpenCommandPalette
            | TitleBarIntent::OpenNotifications => Dismiss,
            TitleBarIntent::OpenSettings => DismissAndRun,
            _ => Allow,
        },
        // The All modes menu is one of the menus: its rows, pin toggles and
        // dismiss layer send these. Choosing a mode closes every menu.
        AppEvent::Modes(ModesIntent::Select(_)) => DismissAndRun,
        AppEvent::Modes(_) => Allow,
        // The palette is one of the menus: typing in it, its entries and its
        // dismiss layer send these.
        AppEvent::Palette(_) => Allow,
        // A press on a Sessions row, its cross, the New session button or
        // the start card closes the menus and runs. A middle press on a row
        // under a header menu lands on the menu's dismiss layer instead.
        AppEvent::Tabs(_) => DismissAndRun,
        // The pointer moving over the Files tree leaves the menus open; a
        // press on a row closes them.
        AppEvent::Explorer(ExplorerIntent::NodeHovered { .. }) => Allow,
        // The broom, the row crosses and the dismiss layer of the
        // notifications list. The cross of a toast under a menu is a press
        // outside it, so it only closes the menus.
        AppEvent::Notifications(
            NotificationsIntent::Clear
            | NotificationsIntent::Remove(_)
            | NotificationsIntent::CloseList,
        ) => Allow,
        #[cfg(not(target_os = "macos"))]
        AppEvent::ResizeWindow(_) => Allow,
        // A shortcut closes the menus and runs. Other keys no widget took do
        // nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
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
/// then the overlays of the active mode: in Terminal mode the pane menu of
/// the terminal, the context menu of the launches and the inline edit, in
/// Containers mode a create dialog, the confirmation dialog and the row
/// menu. Those of the other mode are not on screen, so they do not count.
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
    }
}

/// Return `true` when the event is one of the header menu items: New
/// terminal, Split and the launches of the New session menu, and New launch
/// and New folder of the Launches menu. New launch of the New session menu
/// is a command, which closes the menus and runs anyway.
///
/// The frame closes the header menu and runs these instead of asking
/// [`context_menu_guard`], which would drop a choice that is not a press
/// outside the menu.
pub(crate) fn is_header_menu_choice(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Tabs(TabsIntent::OpenTerminalTab)
            | AppEvent::Terminal(TerminalIntent::SplitPane { .. })
            | AppEvent::QuickLaunch(
                QuickLaunchIntent::LaunchNode { .. }
                    | QuickLaunchIntent::HeaderCreateCommand
                    | QuickLaunchIntent::HeaderCreateFolder
            )
    )
}

/// Return `true` when an inline edit should be cancelled before the input
/// runs: anything but typing into the name field, pointer moves, terminal
/// events and key presses. A shortcut cancels it: it arrives as a command.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunch(intent) => !matches!(
            intent,
            QuickLaunchIntent::InlineEditChanged(_)
                | QuickLaunchIntent::InlineEditSubmit
                | QuickLaunchIntent::CursorMoved { .. }
                | QuickLaunchIntent::NodeHovered { .. }
        ),
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { .. })
        | AppEvent::Terminal(
            TerminalIntent::Widget(_)
            | TerminalIntent::PaneGridCursorMoved { .. },
        )
        | AppEvent::Explorer(ExplorerIntent::NodeHovered { .. })
        | AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar)
        | AppEvent::Keyboard(_) => false,
        _ => true,
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

/// Return `true` when the event is a press of the Esc key that no widget
/// took. The frame closes the overlay on top with it. A held key repeats its
/// press; the repeats do not count, so one press closes one overlay.
pub(crate) fn is_escape_press(event: &AppEvent) -> bool {
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event, Key};

    matches!(
        event,
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            repeat: false,
            ..
        })
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
            | AppEvent::Explorer(ExplorerIntent::NodeHovered { .. })
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

/// The open overlay of Containers mode on top: a dialog lies over the row
/// menu, and a create dialog over the confirmation.
fn containers_overlay(app: &App) -> Option<Overlay> {
    if app.containers.has_create_dialog() {
        return Some(Overlay::ContainersCreate);
    }
    if app.containers.has_confirm() {
        return Some(Overlay::ContainersConfirm);
    }
    if app.containers.has_row_menu() {
        return Some(Overlay::ContainersRowMenu);
    }

    None
}

/// The open overlay of Terminal mode on top: a context menu lies over the
/// inline edit.
fn terminal_overlay(app: &App) -> Option<Overlay> {
    if app.terminal.has_context_menu() {
        return Some(Overlay::TerminalContextMenu);
    }
    if app.quick_launch.has_context_menu() {
        return Some(Overlay::LaunchContextMenu);
    }
    if app.quick_launch.has_inline_edit() {
        return Some(Overlay::InlineEdit);
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
    use iced::keyboard::key::{Code, Named, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};
    use otty_app_domain::commands::AppCommand;
    use otty_app_explorer::ExplorerIntent;
    use otty_app_slots::Component;
    use otty_ui_kit::components::command_palette::PaletteAction;

    use super::*;
    use crate::frame::sidebar::types::HeaderMenu;
    use crate::testing::keyboard::{escape_press, key_press};
    use crate::testing::window::{
        app, containers_app_with_lists, open_confirm, open_create_dialog,
        open_launch_menu, open_notification_list, open_palette, open_pane_menu,
        open_row_menu, open_secret_prompt, open_settings, open_terminal_tab,
        start_inline_edit,
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

    /// The dialogs: they wait for their answer.
    const DIALOGS: [Overlay; 4] = [
        Overlay::SecretPrompt,
        Overlay::Settings,
        Overlay::ContainersCreate,
        Overlay::ContainersConfirm,
    ];

    #[test]
    fn each_overlay_is_a_menu_a_dialog_or_the_inline_edit() {
        for (_, overlay) in OVERLAYS.into_iter().chain(CONTAINERS_OVERLAYS) {
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

        assert!(app.containers.has_row_menu());
        assert_eq!(top_overlay(&app), Some(Overlay::ContainersCreate));
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
    fn dialogs_with_answers() -> [(Overlay, Vec<AppEvent>); 4] {
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
        ]
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
            AppEvent::TitleBar(TitleBarIntent::ToggleFilesPanel),
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
            AppEvent::Tabs(TabsIntent::OpenTerminalTab),
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

    /// The items of the pane menu and what the terminals report pass under
    /// a menu; a press on a pane does nothing, and another terminal press
    /// closes the menus.
    #[test]
    fn the_pane_menu_takes_its_items_and_ignores_a_pane_press() {
        let (_, pane) = iced::widget::pane_grid::State::new(());
        let items = [
            TerminalIntent::CloseContextMenu { tab_id: 1 },
            TerminalIntent::CopySelection {
                tab_id: 1,
                terminal_id: 2,
            },
            TerminalIntent::PasteIntoPrompt {
                tab_id: 1,
                terminal_id: 2,
            },
            TerminalIntent::SplitPane {
                tab_id: 1,
                pane,
                axis: iced::widget::pane_grid::Axis::Vertical,
            },
            TerminalIntent::ClosePane { tab_id: 1, pane },
            TerminalIntent::Widget(otty_ui_term::Event::Redraw { id: 2 }),
        ];
        let ignored = [
            TerminalIntent::PaneClicked { tab_id: 1, pane },
            TerminalIntent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 2,
            },
        ];

        for intent in items {
            let guard = context_menu_guard(&AppEvent::Terminal(intent));

            assert_eq!(guard, MenuGuard::Allow);
        }
        for intent in ignored {
            let guard = context_menu_guard(&AppEvent::Terminal(intent));

            assert_eq!(guard, MenuGuard::Ignore);
        }
        assert_eq!(
            context_menu_guard(&AppEvent::Terminal(
                TerminalIntent::PaneResized {
                    tab_id: 1,
                    event: iced::widget::pane_grid::ResizeEvent {
                        split: split_of_two_panes(),
                        ratio: 0.5,
                    },
                }
            )),
            MenuGuard::Dismiss
        );
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
    /// menu closes the menus and runs.
    #[test]
    fn a_tabs_press_under_a_menu_closes_the_menus_and_goes_on() {
        for intent in [
            TabsIntent::ActivateTab { tab_id: 1 },
            TabsIntent::CloseTab { tab_id: 1 },
            TabsIntent::OpenTerminalTab,
        ] {
            let guard = context_menu_guard(&AppEvent::Tabs(intent));

            assert_eq!(guard, MenuGuard::DismissAndRun);
        }
    }

    /// The title of the Sessions section is a sidebar press: under a menu
    /// it only closes the menus.
    #[test]
    fn the_sessions_title_under_a_menu_only_closes_the_menus() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarIntent::ToggleSessions,
        ));

        assert_eq!(guard, MenuGuard::Dismiss);
    }

    #[test]
    fn each_header_menu_item_is_a_header_menu_choice() {
        let open_terminal = AppEvent::Tabs(TabsIntent::OpenTerminalTab);
        let (_, pane) = iced::widget::pane_grid::State::new(());
        let split = AppEvent::Terminal(TerminalIntent::SplitPane {
            tab_id: 1,
            pane,
            axis: iced::widget::pane_grid::Axis::Vertical,
        });
        let launch = AppEvent::QuickLaunch(QuickLaunchIntent::LaunchNode {
            path: vec![String::from("Demo")],
        });
        let new_launch =
            AppEvent::QuickLaunch(QuickLaunchIntent::HeaderCreateCommand);
        let new_folder =
            AppEvent::QuickLaunch(QuickLaunchIntent::HeaderCreateFolder);

        for choice in [open_terminal, split, launch, new_launch, new_folder] {
            assert!(is_header_menu_choice(&choice), "{choice:?}");
        }
    }

    #[test]
    fn closing_a_tab_or_pressing_a_launch_is_no_header_menu_choice() {
        let close_tab = AppEvent::Tabs(TabsIntent::CloseTab { tab_id: 1 });
        let pressed = AppEvent::QuickLaunch(QuickLaunchIntent::NodePressed {
            path: vec![String::from("Demo")],
        });

        assert!(!is_header_menu_choice(&close_tab));
        assert!(!is_header_menu_choice(&pressed));
        assert!(!is_header_menu_choice(&AppEvent::Sidebar(
            SidebarIntent::HeaderMenuDismiss
        )));
    }

    /// The items and the dismiss layer of the Launches menu, the name field
    /// and the pointer over the tree pass under a menu; other presses in
    /// the tree do nothing.
    #[test]
    fn the_launch_menu_takes_its_items_and_ignores_other_tree_presses() {
        let passing = [
            QuickLaunchIntent::ContextMenuDismiss,
            QuickLaunchIntent::ContextMenuAction(
                otty_app_quick_launch::ContextMenuAction::Rename,
            ),
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
            QuickLaunchIntent::NodeRightClicked {
                path: vec![String::from("Demo")],
            },
        ];

        for intent in passing {
            let guard = context_menu_guard(&AppEvent::QuickLaunch(intent));
            assert_eq!(guard, MenuGuard::Allow);
        }
        for intent in ignored {
            let guard = context_menu_guard(&AppEvent::QuickLaunch(intent));
            assert_eq!(guard, MenuGuard::Ignore);
        }
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
            AppEvent::Command(AppCommand::ToggleFilesPanel),
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
    fn a_pressed_escape_key_is_an_escape_press() {
        assert!(is_escape_press(&AppEvent::Keyboard(escape_press())));
    }

    /// A held Esc closes one overlay, not the next one under it too.
    #[test]
    fn a_held_esc_is_not_an_escape_press() {
        let held = AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: true,
        });

        assert!(!is_escape_press(&held));
    }

    #[test]
    fn other_keys_and_a_released_escape_are_not_an_escape_press() {
        let letter = AppEvent::Keyboard(key_press(
            Physical::Unidentified(NativeCode::Unidentified),
            "a",
            Modifiers::empty(),
        ));
        let released = AppEvent::Keyboard(Event::KeyReleased {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
        });

        assert!(!is_escape_press(&letter));
        assert!(!is_escape_press(&released));
        assert!(!is_escape_press(&AppEvent::HostNameRead(None)));
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

    /// The items of the row menu and its dismiss layer go to the open menu.
    #[test]
    fn the_row_menu_s_items_and_dismiss_pass() {
        use otty_app_containers::RowAction;

        let events = [
            ActionsIntent::Act(RowAction::Stop),
            ActionsIntent::Act(RowAction::Remove),
            ActionsIntent::CloseRowMenu,
        ]
        .map(|intent| AppEvent::Containers(ContainersIntent::Actions(intent)));

        for event in events {
            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    /// Typing, choosing an entry and the palette's own dismiss layer go to
    /// the open palette.
    #[test]
    fn the_palette_s_own_input_passes() {
        let intents = [
            PaletteIntent::Action(PaletteAction::QueryChanged(String::from(
                "spl",
            ))),
            PaletteIntent::Action(PaletteAction::MoveDown),
            PaletteIntent::Action(PaletteAction::RunActive),
            PaletteIntent::FocusInput,
            PaletteIntent::Dismiss,
        ];

        for intent in intents {
            let event = AppEvent::Palette(intent);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    /// The broom, the cross of a row and the list's own dismiss layer go to
    /// the open list.
    #[test]
    fn the_notification_list_s_own_input_passes() {
        for intent in [
            NotificationsIntent::Clear,
            NotificationsIntent::Remove(1),
            NotificationsIntent::CloseList,
        ] {
            let event = AppEvent::Notifications(intent);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    /// The cross of a toast under a menu is a click outside the menu: it
    /// only closes the menu.
    #[test]
    fn a_toast_cross_under_a_menu_only_closes_the_menus() {
        let event =
            AppEvent::Notifications(NotificationsIntent::DismissToast(1));

        assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
    }

    #[test]
    fn given_menu_open_when_modes_events_arrive_then_they_are_allowed() {
        let events = [
            AppEvent::Modes(ModesIntent::CloseMenu),
            AppEvent::Modes(ModesIntent::Pin(Mode::Containers)),
        ];

        for event in events {
            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn choosing_a_mode_or_opening_settings_closes_the_menus_and_goes_on() {
        let events = [
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
            AppEvent::TitleBar(TitleBarIntent::OpenSettings),
        ];

        for event in events {
            assert_eq!(context_menu_guard(&event), MenuGuard::DismissAndRun);
        }
    }

    #[test]
    fn a_panel_or_palette_button_only_closes_the_menus() {
        let presses = [
            TitleBarIntent::ToggleLeftPanel,
            TitleBarIntent::ToggleFilesPanel,
            TitleBarIntent::ToggleFocusMode,
            TitleBarIntent::OpenCommandPalette,
            TitleBarIntent::OpenNotifications,
        ];

        for press in presses {
            let event = AppEvent::TitleBar(press);

            assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
        }
    }

    #[test]
    fn a_window_button_leaves_the_menus_open() {
        for press in [
            TitleBarIntent::MinimizeWindow,
            TitleBarIntent::ToggleMaximize,
            TitleBarIntent::CloseWindow,
        ] {
            let event = AppEvent::TitleBar(press);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn moving_over_the_title_bar_keeps_the_menus() {
        let event = AppEvent::TitleBar(TitleBarIntent::MoveOnTitleBar);

        assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
    }

    #[test]
    fn given_menu_open_when_menu_open_intent_arrives_then_it_is_ignored() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        ));

        assert_eq!(guard, MenuGuard::Ignore);
    }

    #[test]
    fn given_menu_open_when_menu_dismiss_intent_arrives_then_it_is_allowed() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarIntent::HeaderMenuDismiss,
        ));

        assert_eq!(guard, MenuGuard::Allow);
    }

    #[test]
    fn the_right_panel_toggle_closes_the_menus() {
        let toggle = AppEvent::Sidebar(SidebarIntent::ToggleRight);

        assert_eq!(context_menu_guard(&toggle), MenuGuard::Dismiss);
    }

    #[test]
    fn hovering_a_files_row_keeps_the_menus() {
        // A row sends the path when the pointer enters it, and no path when
        // the pointer leaves it or the whole tree.
        let paths = [Some(vec![String::from("src")]), None];

        for path in paths {
            let event =
                AppEvent::Explorer(ExplorerIntent::NodeHovered { path });

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn pressing_a_files_row_still_closes_the_menus() {
        let event = AppEvent::Explorer(ExplorerIntent::NodePressed {
            path: vec![String::from("src")],
        });

        assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn a_window_grip_leaves_the_menus_open() {
        let grip = AppEvent::ResizeWindow(iced::window::Direction::East);

        assert_eq!(context_menu_guard(&grip), MenuGuard::Allow);
    }

    #[test]
    fn a_command_under_an_open_menu_closes_the_menus_and_goes_on() {
        let event = AppEvent::Command(AppCommand::ToggleFocusMode);

        assert_eq!(context_menu_guard(&event), MenuGuard::DismissAndRun);
    }

    #[test]
    fn a_key_that_is_not_a_shortcut_does_nothing_under_a_menu() {
        let event = AppEvent::Keyboard(key_press(
            Physical::Code(Code::KeyA),
            "a",
            Modifiers::empty(),
        ));

        assert_eq!(context_menu_guard(&event), MenuGuard::Ignore);
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
            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
            assert!(inline_edit_guard(&event));
        }
    }
}
