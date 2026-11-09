//! What input from the user does while a menu or a dialog is open or an
//! inline edit runs, which overlay lies on top of the window, and what
//! decides whether a command can run now. The input checks only see
//! input: what follows from it, and what arrives on its own, skips them (see
//! `update_internal` in `update.rs`), so it can not get lost.

use crate::app::App;
use crate::commands::CommandCtx;
use crate::domain::mode::Mode;
use crate::event::AppEvent;
use crate::widgets::chrome::event::{ChromeEvent, ChromeIntent};
use crate::widgets::containers::child::actions::event::ActionsIntent;
use crate::widgets::containers::child::create::event::CreateIntent;
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::notifications::event::{
    NotificationsEvent, NotificationsIntent,
};
use crate::widgets::palette::event::{PaletteEvent, PaletteIntent};
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::settings::event::{SettingsEvent, SettingsIntent};
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// An overlay: a menu, a dialog or the inline edit that lies over the window
/// and takes the user's input before the window under it. [`top_overlay`]
/// finds the one on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    /// The SSH prompt that asks for a password or a passphrase.
    SecretPrompt,
    /// The Settings popup, over every mode.
    Settings,
    /// The notifications list under the bell, over every mode.
    NotificationsList,
    /// The command palette.
    Palette,
    /// The All modes menu.
    ModesMenu,
    /// The New session or Launches menu of a section header.
    HeaderMenu,
    /// A Containers create dialog, such as Pull image.
    ContainersCreate,
    /// The Containers dialog that asks to confirm a destructive action.
    ContainersConfirm,
    /// The menu of a Containers row.
    ContainersRowMenu,
    /// The context menu of a terminal pane.
    TerminalContextMenu,
    /// The context menu of the Launches tree.
    LaunchContextMenu,
    /// A name typed in the Launches tree: a new folder or a rename.
    InlineEdit,
}

impl Overlay {
    /// Return `true` for a menu: the notifications list, the palette, the
    /// All modes menu, the header menus, the Containers row menu and the
    /// context menus. A click outside a menu closes it, and a shortcut
    /// closes it and runs.
    pub(crate) fn is_menu(self) -> bool {
        matches!(
            self,
            Overlay::NotificationsList
                | Overlay::Palette
                | Overlay::ModesMenu
                | Overlay::HeaderMenu
                | Overlay::ContainersRowMenu
                | Overlay::TerminalContextMenu
                | Overlay::LaunchContextMenu
        )
    }

    /// Return `true` for a dialog: the SSH prompt, Settings and the
    /// Containers create and confirm dialogs. A dialog waits for its answer.
    pub(crate) fn is_dialog(self) -> bool {
        matches!(
            self,
            Overlay::SecretPrompt
                | Overlay::Settings
                | Overlay::ContainersCreate
                | Overlay::ContainersConfirm
        )
    }

    /// Return what Esc sends while this overlay is on top: the event that
    /// closes a menu, or Cancel of a dialog and of the inline edit.
    pub(crate) fn escape_event(self) -> AppEvent {
        match self {
            Overlay::SecretPrompt => {
                quick_launch(QuickLaunchIntent::SecretPromptCancel)
            },
            Overlay::Settings => settings(SettingsIntent::Close),
            Overlay::NotificationsList => {
                notifications(NotificationsIntent::CloseList)
            },
            Overlay::Palette => {
                AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Dismiss))
            },
            Overlay::ModesMenu => {
                AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu))
            },
            Overlay::HeaderMenu => AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss,
            )),
            Overlay::ContainersCreate => {
                containers(ContainersIntent::Create(CreateIntent::Cancelled))
            },
            Overlay::ContainersConfirm => containers(
                ContainersIntent::Actions(ActionsIntent::ConfirmCancelled),
            ),
            Overlay::ContainersRowMenu => containers(
                ContainersIntent::Actions(ActionsIntent::CloseRowMenu),
            ),
            Overlay::TerminalContextMenu => {
                AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::CloseAllContextMenus,
                ))
            },
            Overlay::LaunchContextMenu => {
                quick_launch(QuickLaunchIntent::ContextMenuDismiss)
            },
            Overlay::InlineEdit => {
                quick_launch(QuickLaunchIntent::CancelInlineEdit)
            },
        }
    }
}

/// Determines how the event loop should treat input from the user when a
/// menu or a dialog is on top.
#[derive(Debug, Clone, Copy)]
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
        AppEvent::Sidebar(SidebarEvent::Intent(event)) => {
            use SidebarIntent as E;
            match event {
                E::HeaderMenuDismiss | E::WorkspaceCursorMoved { .. } => Allow,
                E::HeaderMenuOpen(_) => Ignore,
                _ => Dismiss,
            }
        },
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event)) => {
            use QuickLaunchIntent as E;
            match event {
                E::ContextMenuDismiss
                | E::ContextMenuAction(_)
                | E::InlineEditChanged(_)
                | E::InlineEditSubmit
                | E::CursorMoved { .. } => Allow,
                _ => Ignore,
            }
        },
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(event)) => {
            use TerminalWorkspaceIntent as E;
            match event {
                E::CloseContextMenu { .. }
                | E::CopySelection { .. }
                | E::PasteIntoPrompt { .. }
                | E::CopySelectedBlockContent { .. }
                | E::CopySelectedBlockPrompt { .. }
                | E::CopySelectedBlockCommand { .. }
                | E::SplitPane { .. }
                | E::ClosePane { .. }
                | E::Widget(_)
                | E::PaneGridCursorMoved { .. } => Allow,
                E::OpenContextMenu { .. } | E::PaneClicked { .. } => Ignore,
                _ => Dismiss,
            }
        },
        // The first press on a panel button, the palette button or the bell
        // only closes the menu, the notifications list too; opening settings
        // closes it and goes on.
        AppEvent::Chrome(ChromeEvent::Intent(event)) => {
            use ChromeIntent as E;
            match event {
                E::ToggleLeftPanel
                | E::ToggleFilesPanel
                | E::ToggleFocusMode
                | E::OpenCommandPalette
                | E::OpenNotifications => Dismiss,
                E::OpenSettings => DismissAndRun,
                _ => Allow,
            }
        },
        AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. },
        )) => Allow,
        // The palette is one of the menus: typing in it, its entries and its
        // dismiss layer send these.
        AppEvent::Palette(_) => Allow,
        // The broom, the row crosses and the dismiss layer of the
        // notifications list.
        AppEvent::Notifications(NotificationsEvent::Intent(
            NotificationsIntent::Clear
            | NotificationsIntent::Remove(_)
            | NotificationsIntent::CloseList,
        )) => Allow,
        // The items and the dismiss layer of the Containers row menu.
        AppEvent::Containers(ContainersEvent::Intent(
            ContainersIntent::Actions(
                ActionsIntent::Act(_) | ActionsIntent::CloseRowMenu,
            ),
        )) => Allow,
        AppEvent::Tabs(_) => DismissAndRun,
        // The All modes menu is one of the menus: its rows, pin toggles and
        // dismiss layer send these. Choosing a mode closes every menu.
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(_))) => {
            DismissAndRun
        },
        AppEvent::Modes(_) => Allow,
        AppEvent::ResizeWindow(_) => Allow,
        // A shortcut, whichever key path it took, closes the menus and
        // runs. Other keys no widget took do nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// Classify input from the user while `dialog` is on top: it returns only
/// `Allow` or `Ignore`. A dialog waits for its answer, so only its own
/// answers, pointer moves, window controls and terminal reports pass.
/// Shortcuts, app buttons, presses outside it and other keys do nothing,
/// and a menu under the dialog stays open.
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
/// then the overlays of the active mode. Those of the other mode are not on
/// screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
    let widgets = &app.widgets;

    if widgets.quick_launch.has_secret_prompt() {
        return Some(Overlay::SecretPrompt);
    }
    if widgets.settings.is_open() {
        return Some(Overlay::Settings);
    }
    if widgets.notifications.is_list_open() {
        return Some(Overlay::NotificationsList);
    }
    if widgets.palette.is_open() {
        return Some(Overlay::Palette);
    }
    if widgets.modes.is_menu_open() {
        return Some(Overlay::ModesMenu);
    }
    if widgets.sidebar.has_header_menu_open() {
        return Some(Overlay::HeaderMenu);
    }

    match widgets.modes.active() {
        Mode::Containers => containers_overlay(app),
        Mode::Terminal => terminal_overlay(app),
    }
}

/// Gather what decides whether a command can run now: the mode on screen,
/// the active tab and its focused pane. The input filter checks shortcuts
/// with it, and the palette lists and runs only the commands it allows (see
/// [`crate::commands::AppCommand::is_available`]).
pub(crate) fn command_ctx(app: &App) -> CommandCtx {
    let active_tab_id = app.widgets.tabs.active_tab_id();

    CommandCtx {
        is_terminal_mode: app.widgets.modes.active() == Mode::Terminal,
        has_active_tab: active_tab_id.is_some(),
        has_focused_pane: app
            .widgets
            .terminal_workspace
            .focused_pane(active_tab_id)
            .is_some(),
    }
}

/// Return `true` when the event is one of the header menu items (New session
/// or Launches): open a terminal, split the pane, run a launch, create a
/// launch, or create a folder.
///
/// The app closes the menu and runs these instead of asking the context
/// menu guard, which would drop the launch intents.
pub(crate) fn is_header_menu_choice(event: &AppEvent) -> bool {
    use crate::widgets::quick_launch::event::{
        QuickLaunchEvent, QuickLaunchIntent,
    };
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::terminal_workspace::event::{
        TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    matches!(
        event,
        AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab))
            | AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane { .. }
            ))
            | AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::LaunchNode { .. }
                    | QuickLaunchIntent::HeaderCreateCommand
                    | QuickLaunchIntent::HeaderCreateFolder
            ))
    )
}

/// Return `true` when the event is a press of the Esc key that no widget
/// took. The app closes the overlay on top with it. A held key repeats its
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

/// Return `true` when an active inline-edit should be cancelled before the
/// input is dispatched: anything but typing into the edit, pointer moves,
/// terminal events and key presses. A shortcut cancels it: it arrives as a
/// command.
pub(crate) fn inline_edit_guard(event: &AppEvent) -> bool {
    match event {
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event)) => {
            use QuickLaunchIntent as E;
            !matches!(
                event,
                E::InlineEditChanged(_)
                    | E::InlineEditSubmit
                    | E::CursorMoved { .. }
                    | E::NodeHovered { .. }
            )
        },
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { .. },
        )) => false,
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(_)
            | TerminalWorkspaceIntent::PaneGridCursorMoved { .. },
        )) => false,
        AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. },
        ))
        | AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MoveOnTitleBar)) => {
            false
        },
        AppEvent::Keyboard(_) => false,
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
    let covers_terminals =
        top.is_some_and(|top| top.is_menu() || top.is_dialog());

    covers_terminals && is_terminal_input(event)
}

/// The open overlay of Containers mode on top: a dialog lies over the row
/// menu.
fn containers_overlay(app: &App) -> Option<Overlay> {
    let containers = &app.widgets.containers;

    if containers.has_create_dialog() {
        return Some(Overlay::ContainersCreate);
    }
    if containers.has_confirm() {
        return Some(Overlay::ContainersConfirm);
    }
    if containers.has_row_menu() {
        return Some(Overlay::ContainersRowMenu);
    }

    None
}

/// The open overlay of Terminal mode on top: a context menu lies over the
/// inline edit.
fn terminal_overlay(app: &App) -> Option<Overlay> {
    let widgets = &app.widgets;

    if widgets.terminal_workspace.has_any_context_menu() {
        return Some(Overlay::TerminalContextMenu);
    }
    if widgets.quick_launch.has_context_menu() {
        return Some(Overlay::LaunchContextMenu);
    }
    if widgets.quick_launch.has_inline_edit() {
        return Some(Overlay::InlineEdit);
    }

    None
}

/// Whether `event` answers `dialog`: typing in it, its buttons, Enter and
/// Esc. For Settings these are its fields, choices and buttons and the
/// close cross.
fn is_dialog_answer(dialog: Overlay, event: &AppEvent) -> bool {
    match dialog {
        Overlay::SecretPrompt => matches!(
            event,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::SecretPromptChanged(_)
                    | QuickLaunchIntent::SecretPromptSubmit
                    | QuickLaunchIntent::SecretPromptCancel
            ))
        ),
        Overlay::Settings => matches!(
            event,
            AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::ThemeSelected(_)
                    | SettingsIntent::LanguageChanged(_)
                    | SettingsIntent::ShellChanged(_)
                    | SettingsIntent::EditorChanged(_)
                    | SettingsIntent::EqualizePanesToggled(_)
                    | SettingsIntent::Save
                    | SettingsIntent::Reset
                    | SettingsIntent::Close
            ))
        ),
        Overlay::ContainersConfirm => matches!(
            event,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Actions(
                    ActionsIntent::ConfirmSwitchToggled(_)
                        | ActionsIntent::ConfirmAccepted
                        | ActionsIntent::ConfirmCancelled
                )
            ))
        ),
        Overlay::ContainersCreate => matches!(
            event,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Create(
                    CreateIntent::Changed(..)
                        | CreateIntent::Submitted
                        | CreateIntent::Cancelled
                )
            ))
        ),
        _ => false,
    }
}

/// Whether `event` passes under any dialog: a pointer move, a window
/// control or a report of a terminal.
fn passes_every_dialog(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { .. }
        )) | AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::PaneGridCursorMoved { .. }
                | TerminalWorkspaceIntent::Widget(_)
        )) | AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::CursorMoved { .. }
                | QuickLaunchIntent::NodeHovered { .. }
        )) | AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. }
        )) | AppEvent::Chrome(ChromeEvent::Intent(
            ChromeIntent::ToggleMaximize
                | ChromeIntent::MinimizeWindow
                | ChromeIntent::CloseWindow
                | ChromeIntent::PressTitleBar
                | ChromeIntent::MoveOnTitleBar
                | ChromeIntent::ReleaseTitleBar
        )) | AppEvent::ResizeWindow(_)
    )
}

fn quick_launch(intent: QuickLaunchIntent) -> AppEvent {
    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
}

fn containers(intent: ContainersIntent) -> AppEvent {
    AppEvent::Containers(ContainersEvent::Intent(intent))
}

fn settings(intent: SettingsIntent) -> AppEvent {
    AppEvent::Settings(SettingsEvent::Intent(intent))
}

fn notifications(intent: NotificationsIntent) -> AppEvent {
    AppEvent::Notifications(NotificationsEvent::Intent(intent))
}

/// Return `true` when the event carries text a terminal sends to its shell:
/// typed keys, a paste or composed input.
fn is_terminal_input(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(otty_ui_term::Event::Write { .. })
        ))
    )
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Named, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};
    use otty_ui_kit::components::command_palette::PaletteAction;

    use super::{
        MenuGuard, Overlay, command_ctx, context_menu_guard, dialog_guard,
        inline_edit_guard, is_escape_press, is_header_menu_choice,
        is_terminal_input, is_terminal_input_kept_out, top_overlay,
    };
    use crate::app::App;
    use crate::commands::{AppCommand, CommandCtx};
    use crate::domain::mode::Mode;
    use crate::event::AppEvent;
    use crate::shortcuts::AppShortcut;
    use crate::testing::overlays::{
        open_confirm, open_create_dialog, open_header_menu, open_launch_menu,
        open_modes_menu, open_notification_list, open_palette, open_pane_menu,
        open_row_menu, open_secret_prompt, open_settings, start_inline_edit,
    };
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::window::{
        app_in, containers_app_with_lists, open_launch_editor, show_toast,
    };
    use crate::widgets::chrome::event::{ChromeEvent, ChromeIntent};
    use crate::widgets::containers::child::actions::event::ActionsIntent;
    use crate::widgets::containers::child::actions::types::RowAction;
    use crate::widgets::containers::child::create::event::CreateIntent;
    use crate::widgets::containers::event::{
        ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::types::InspectorTab;
    use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
    use crate::widgets::modes::event::{ModesEvent, ModesIntent};
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::palette::event::{PaletteEvent, PaletteIntent};
    use crate::widgets::quick_launch::event::{
        QuickLaunchEvent, QuickLaunchIntent,
    };
    use crate::widgets::settings::event::{SettingsEvent, SettingsIntent};
    use crate::widgets::settings::types::LanguageSetting;
    use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
    use crate::widgets::sidebar::types::HeaderMenu;
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::terminal_workspace::event::{
        TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    /// Opens one overlay on an app.
    type Opener = fn(&mut App);

    /// The openers of the overlays of Terminal mode and of both modes, with
    /// the overlay each one opens.
    const TERMINAL_OVERLAYS: [(Opener, Overlay); 9] = [
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

    /// The menus: a click outside closes them.
    const MENUS: [Overlay; 7] = [
        Overlay::NotificationsList,
        Overlay::Palette,
        Overlay::ModesMenu,
        Overlay::HeaderMenu,
        Overlay::ContainersRowMenu,
        Overlay::TerminalContextMenu,
        Overlay::LaunchContextMenu,
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
        let overlays = TERMINAL_OVERLAYS
            .into_iter()
            .chain(CONTAINERS_OVERLAYS)
            .map(|(_, overlay)| overlay);

        for overlay in overlays {
            let kinds = [
                overlay.is_menu(),
                overlay.is_dialog(),
                overlay == Overlay::InlineEdit,
            ];

            assert_eq!(
                kinds.iter().filter(|is| **is).count(),
                1,
                "{overlay:?}"
            );
        }
        for menu in MENUS {
            assert!(menu.is_menu(), "{menu:?}");
        }
        for dialog in DIALOGS {
            assert!(dialog.is_dialog(), "{dialog:?}");
        }
    }

    /// Scenario 2: typing under a menu or a dialog never reaches the shell
    /// behind it, even if its terminal still had keyboard focus.
    #[test]
    fn terminal_input_is_kept_out_under_every_menu_and_dialog_but_not_an_inline_edit()
     {
        let typed = terminal_widget_event(otty_ui_term::Event::Write {
            id: 1,
            data: b"ls".to_vec(),
        });

        for (open, overlay) in TERMINAL_OVERLAYS {
            let mut app = app_in(Mode::Terminal);
            open(&mut app);

            let is_kept_out =
                is_terminal_input_kept_out(top_overlay(&app), &typed);

            assert_eq!(
                is_kept_out,
                overlay != Overlay::InlineEdit,
                "{overlay:?}"
            );
        }
        for (open, overlay) in CONTAINERS_OVERLAYS {
            let mut app = containers_app_with_lists();
            open(&mut app);

            let is_kept_out =
                is_terminal_input_kept_out(top_overlay(&app), &typed);

            assert!(is_kept_out, "{overlay:?}");
        }
    }

    #[test]
    fn terminal_input_passes_with_nothing_open_and_a_report_always_passes() {
        let typed = terminal_widget_event(otty_ui_term::Event::Write {
            id: 1,
            data: b"ls".to_vec(),
        });
        let redraw =
            terminal_widget_event(otty_ui_term::Event::Redraw { id: 1 });

        assert!(!is_terminal_input_kept_out(None, &typed));
        assert!(!is_terminal_input_kept_out(
            Some(Overlay::HeaderMenu),
            &redraw
        ));
    }

    #[test]
    fn the_command_context_reads_the_mode_the_active_tab_and_its_pane() {
        let mut app = app_in(Mode::Terminal);
        let without_tabs = command_ctx(&app);
        let _ = open_launch_editor(&mut app);
        let with_launch_editor = command_ctx(&app);
        let _ = open_shell_tab(&mut app);
        let with_shell = command_ctx(&app);
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));
        let in_containers = command_ctx(&app);

        assert_eq!(
            without_tabs,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            }
        );
        assert_eq!(
            with_launch_editor,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: false,
            }
        );
        assert_eq!(
            with_shell,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: true,
            }
        );
        assert!(!in_containers.is_terminal_mode);
    }

    #[test]
    fn nothing_is_on_top_of_a_plain_window() {
        let terminal = app_in(Mode::Terminal);
        let containers = containers_app_with_lists();

        assert_eq!(top_overlay(&terminal), None);
        assert_eq!(top_overlay(&containers), None);
    }

    /// A toast is no overlay: it takes no keys and Esc leaves it.
    #[test]
    fn a_toast_is_no_overlay() {
        let mut app = app_in(Mode::Terminal);

        show_toast(&mut app, "disk full");

        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn each_open_overlay_is_found() {
        for (open, overlay) in TERMINAL_OVERLAYS {
            let mut app = app_in(Mode::Terminal);
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
        }
        for (open, overlay) in CONTAINERS_OVERLAYS {
            let mut app = containers_app_with_lists();
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
        }
    }

    /// Scenario 14: an SSH launch asks for its password while a menu is open.
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
            let mut app = app_in(Mode::Terminal);
            open_menu(&mut app);
            open_secret_prompt(&mut app);

            assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
        }
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);
        open_secret_prompt(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
    }

    /// An SSH launch asks for its password while Settings is open.
    #[test]
    fn the_secret_prompt_lies_over_settings() {
        let mut app = app_in(Mode::Terminal);
        open_settings(&mut app);
        open_secret_prompt(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::SecretPrompt));
    }

    /// The list lies under Settings and over the palette, and counts in
    /// Containers mode too, over the overlays of the mode.
    #[test]
    fn the_notification_list_lies_under_settings_and_over_the_palette() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);
        open_palette(&mut app);
        open_notification_list(&mut app);
        let over_the_palette = top_overlay(&app);

        open_settings(&mut app);

        assert_eq!(over_the_palette, Some(Overlay::NotificationsList));
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// Settings lies over every mode, so it counts in Containers mode too,
    /// over the overlays of the mode.
    #[test]
    fn settings_lies_over_the_overlays_of_containers_mode() {
        let mut app = containers_app_with_lists();
        open_row_menu(&mut app);
        open_settings(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// Scenario 11: a Containers dialog stays open, hidden, while Terminal
    /// mode is shown, and comes back with its mode (owner, Q3).
    #[test]
    fn a_containers_overlay_counts_only_in_containers_mode() {
        let mut app = containers_app_with_lists();
        open_confirm(&mut app);

        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Terminal));
        let in_terminal = top_overlay(&app);
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));

        assert_eq!(in_terminal, None);
        assert_eq!(top_overlay(&app), Some(Overlay::ContainersConfirm));
    }

    #[test]
    fn a_terminal_overlay_counts_only_in_terminal_mode() {
        let mut app = app_in(Mode::Terminal);
        open_launch_menu(&mut app);

        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));

        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn the_inline_edit_is_under_every_other_overlay() {
        let others = TERMINAL_OVERLAYS
            .into_iter()
            .filter(|(_, overlay)| *overlay != Overlay::InlineEdit);

        for (open, overlay) in others {
            let mut app = app_in(Mode::Terminal);
            start_inline_edit(&mut app);
            open(&mut app);

            assert!(app.widgets.quick_launch.has_inline_edit(), "{overlay:?}");
            assert_eq!(top_overlay(&app), Some(overlay));
        }
    }

    fn terminal_widget_event(
        event: otty_ui_term::Event<AppShortcut>,
    ) -> AppEvent {
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::Widget(event),
        ))
    }

    #[test]
    fn text_a_terminal_writes_to_its_shell_is_terminal_input() {
        let event = terminal_widget_event(otty_ui_term::Event::Write {
            id: 1,
            data: b"ls".to_vec(),
        });

        assert!(is_terminal_input(&event));
    }

    #[test]
    fn other_terminal_and_app_events_are_not_terminal_input() {
        let events = [
            terminal_widget_event(otty_ui_term::Event::Redraw { id: 1 }),
            terminal_widget_event(
                otty_ui_term::Event::BindingActionDispatched {
                    id: 1,
                    action: AppShortcut::CommandPalette,
                },
            ),
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            )),
            AppEvent::SyncTerminalGridSizes,
        ];

        for event in events {
            assert!(!is_terminal_input(&event));
        }
    }

    fn key_press(key: Key, physical_key: Physical) -> AppEvent {
        AppEvent::Keyboard(Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn a_pressed_escape_key_is_an_escape_press() {
        let event =
            key_press(Key::Named(Named::Escape), Physical::Code(Code::Escape));

        assert!(is_escape_press(&event));
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
    fn esc_sends_the_close_or_cancel_intent_of_each_overlay() {
        let containers =
            |event: &AppEvent, expected: &ContainersIntent| -> bool {
                matches!(
                    event,
                    AppEvent::Containers(ContainersEvent::Intent(intent))
                        if intent == expected
                )
            };

        assert!(matches!(
            Overlay::SecretPrompt.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::SecretPromptCancel
            ))
        ));
        assert!(matches!(
            Overlay::Settings.escape_event(),
            AppEvent::Settings(SettingsEvent::Intent(SettingsIntent::Close))
        ));
        assert!(matches!(
            Overlay::NotificationsList.escape_event(),
            AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::CloseList
            ))
        ));
        assert!(matches!(
            Overlay::Palette.escape_event(),
            AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Dismiss))
        ));
        assert!(matches!(
            Overlay::ModesMenu.escape_event(),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu))
        ));
        assert!(matches!(
            Overlay::HeaderMenu.escape_event(),
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss
            ))
        ));
        assert!(containers(
            &Overlay::ContainersCreate.escape_event(),
            &ContainersIntent::Create(CreateIntent::Cancelled)
        ));
        assert!(containers(
            &Overlay::ContainersConfirm.escape_event(),
            &ContainersIntent::Actions(ActionsIntent::ConfirmCancelled)
        ));
        assert!(containers(
            &Overlay::ContainersRowMenu.escape_event(),
            &ContainersIntent::Actions(ActionsIntent::CloseRowMenu)
        ));
        assert!(matches!(
            Overlay::TerminalContextMenu.escape_event(),
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::CloseAllContextMenus
            ))
        ));
        assert!(matches!(
            Overlay::LaunchContextMenu.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::ContextMenuDismiss
            ))
        ));
        assert!(matches!(
            Overlay::InlineEdit.escape_event(),
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::CancelInlineEdit
            ))
        ));
    }

    #[test]
    fn other_keys_and_a_released_escape_are_not_an_escape_press() {
        let letter = key_press(
            Key::Character("a".into()),
            Physical::Unidentified(NativeCode::Unidentified),
        );
        let released = AppEvent::Keyboard(Event::KeyReleased {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
        });

        assert!(!is_escape_press(&letter));
        assert!(!is_escape_press(&released));
        assert!(!is_escape_press(&AppEvent::SyncTerminalGridSizes));
    }

    #[test]
    fn given_menu_open_when_modes_events_arrive_then_they_are_allowed() {
        let events = [
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu)),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Pin(
                Mode::Containers,
            ))),
        ];

        for event in events {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        }
    }

    #[test]
    fn choosing_a_mode_or_pressing_a_tab_closes_the_menus_and_goes_on() {
        let events = [
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers,
            ))),
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::ActivateTab {
                tab_id: 1,
            })),
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::OpenSettings)),
        ];

        for event in events {
            assert!(matches!(
                context_menu_guard(&event),
                MenuGuard::DismissAndRun
            ));
        }
    }

    #[test]
    fn a_panel_or_palette_button_only_closes_the_menus() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
            ChromeIntent::OpenCommandPalette,
            ChromeIntent::OpenNotifications,
        ];

        for press in presses {
            let event = AppEvent::Chrome(ChromeEvent::Intent(press));

            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        }
    }

    #[test]
    fn a_window_button_leaves_the_menus_open() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MinimizeWindow));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
    }

    #[test]
    fn a_key_press_that_is_not_a_shortcut_keeps_an_inline_edit() {
        // A shortcut key reaches the checks as its command instead.
        let plain =
            key_press(Key::Character("a".into()), Physical::Code(Code::KeyA));

        assert!(!inline_edit_guard(&plain));
    }

    #[test]
    fn a_click_in_the_containers_views_closes_menus_and_inline_edits() {
        let containers =
            |intent| AppEvent::Containers(ContainersEvent::Intent(intent));
        let clicks = [
            containers(ContainersIntent::ToggleEngine(
                otty_containers::EngineAddress::Tcp(String::from("h:1")),
            )),
            containers(ContainersIntent::Refresh),
            containers(ContainersIntent::SelectRow(0)),
            containers(ContainersIntent::SelectInspectorTab(
                InspectorTab::Logs,
            )),
            containers(ContainersIntent::Actions(ActionsIntent::Prune)),
        ];

        for event in clicks {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
            assert!(inline_edit_guard(&event));
        }
    }

    /// Typing, choosing an entry and the palette's own dismiss layer go to
    /// the open palette.
    #[test]
    fn the_palette_s_own_input_passes() {
        let palette = |intent| AppEvent::Palette(PaletteEvent::Intent(intent));
        let events = [
            palette(PaletteIntent::Action(PaletteAction::QueryChanged(
                String::from("spl"),
            ))),
            palette(PaletteIntent::Action(PaletteAction::MoveDown)),
            palette(PaletteIntent::Action(PaletteAction::RunActive)),
            palette(PaletteIntent::FocusInput),
            palette(PaletteIntent::Dismiss),
        ];

        for event in events {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
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
            let event =
                AppEvent::Notifications(NotificationsEvent::Intent(intent));

            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        }
    }

    /// The cross of a toast under a menu is a click outside the menu: it
    /// only closes the menu.
    #[test]
    fn a_toast_cross_under_a_menu_only_closes_the_menus() {
        let event = AppEvent::Notifications(NotificationsEvent::Intent(
            NotificationsIntent::DismissToast(1),
        ));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
    }

    #[test]
    fn the_row_menu_s_items_and_dismiss_pass() {
        let actions = |intent| {
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Actions(intent),
            ))
        };
        let events = [
            actions(ActionsIntent::Act(RowAction::Stop)),
            actions(ActionsIntent::Act(RowAction::Remove)),
            actions(ActionsIntent::CloseRowMenu),
        ];

        for event in events {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        }
    }

    fn quick_launch(intent: QuickLaunchIntent) -> AppEvent {
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
    }

    fn containers(intent: ContainersIntent) -> AppEvent {
        AppEvent::Containers(ContainersEvent::Intent(intent))
    }

    fn confirm(intent: ActionsIntent) -> AppEvent {
        containers(ContainersIntent::Actions(intent))
    }

    fn create(intent: CreateIntent) -> AppEvent {
        containers(ContainersIntent::Create(intent))
    }

    fn settings(intent: SettingsIntent) -> AppEvent {
        AppEvent::Settings(SettingsEvent::Intent(intent))
    }

    /// Each dialog with what the user answers it: typing, its buttons,
    /// Enter and Esc.
    fn dialogs_with_answers() -> [(Overlay, Vec<AppEvent>); 4] {
        use crate::theme::BuiltinTheme;
        use crate::widgets::containers::child::create::types::FormField;

        [
            (
                Overlay::SecretPrompt,
                vec![
                    quick_launch(QuickLaunchIntent::SecretPromptChanged(
                        String::from("pw"),
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
                        LanguageSetting::System,
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

    fn is_allowed(dialog: Overlay, event: &AppEvent) -> bool {
        matches!(dialog_guard(dialog, event), MenuGuard::Allow)
    }

    #[test]
    fn a_dialog_lets_its_own_answers_through() {
        for (dialog, answers) in dialogs_with_answers() {
            for answer in &answers {
                assert!(is_allowed(dialog, answer), "{dialog:?}");
            }
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

    /// Scenario 8: a shortcut, a header button, a press outside the dialog
    /// and a plain key do nothing while a dialog waits for its answer. So
    /// the gear and the Settings shortcut do nothing while Settings is open
    /// (owner, Q3).
    #[test]
    fn a_dialog_ignores_shortcuts_app_buttons_and_keys() {
        let chrome = |intent| AppEvent::Chrome(ChromeEvent::Intent(intent));
        let events = [
            AppEvent::Command(AppCommand::ToggleFocusMode),
            AppEvent::Command(AppCommand::OpenPalette),
            AppEvent::Command(AppCommand::OpenSettings),
            AppEvent::Command(AppCommand::OpenNotifications),
            AppEvent::Command(AppCommand::SelectModeAt { position: 0 }),
            chrome(ChromeIntent::ToggleLeftPanel),
            chrome(ChromeIntent::ToggleFilesPanel),
            chrome(ChromeIntent::ToggleFocusMode),
            chrome(ChromeIntent::OpenSettings),
            chrome(ChromeIntent::OpenCommandPalette),
            chrome(ChromeIntent::OpenNotifications),
            AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::DismissToast(1),
            )),
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::ActivateTab {
                tab_id: 1,
            })),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Terminal,
            ))),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::ToggleRight)),
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
            )),
            AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Open)),
            quick_launch(QuickLaunchIntent::BackgroundPressed),
            containers(ContainersIntent::SelectRow(0)),
            confirm(ActionsIntent::Act(RowAction::Stop)),
            key_press(Key::Character("a".into()), Physical::Code(Code::KeyA)),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(!is_allowed(dialog, event), "{dialog:?}");
            }
        }
    }

    #[test]
    fn a_dialog_keeps_window_controls_pointer_moves_and_terminal_reports() {
        let chrome = |intent| AppEvent::Chrome(ChromeEvent::Intent(intent));
        let events = [
            chrome(ChromeIntent::ToggleMaximize),
            chrome(ChromeIntent::MinimizeWindow),
            chrome(ChromeIntent::CloseWindow),
            chrome(ChromeIntent::PressTitleBar),
            chrome(ChromeIntent::MoveOnTitleBar),
            chrome(ChromeIntent::ReleaseTitleBar),
            AppEvent::ResizeWindow(iced::window::Direction::East),
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::WorkspaceCursorMoved {
                    position: iced::Point::ORIGIN,
                },
            )),
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::PaneGridCursorMoved {
                    tab_id: 1,
                    position: iced::Point::ORIGIN,
                },
            )),
            quick_launch(QuickLaunchIntent::CursorMoved {
                position: iced::Point::ORIGIN,
            }),
            quick_launch(QuickLaunchIntent::NodeHovered {
                path: vec![String::from("Demo")],
            }),
            AppEvent::Explorer(ExplorerEvent::Intent(
                ExplorerIntent::NodeHovered { path: None },
            )),
            terminal_widget_event(otty_ui_term::Event::Redraw { id: 1 }),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(is_allowed(dialog, event), "{dialog:?}");
            }
        }
    }

    #[test]
    fn given_menu_open_when_menu_open_intent_arrives_then_it_is_ignored() {
        let guard =
            context_menu_guard(&AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
            )));
        assert!(matches!(guard, MenuGuard::Ignore));
    }

    #[test]
    fn given_menu_open_when_menu_dismiss_intent_arrives_then_it_is_allowed() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarEvent::Intent(SidebarIntent::HeaderMenuDismiss),
        ));
        assert!(matches!(guard, MenuGuard::Allow));
    }

    #[test]
    fn given_menu_choices_when_checked_then_each_one_is_a_header_menu_choice() {
        let open_terminal =
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab));
        let launch = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::LaunchNode {
                path: vec![String::from("Demo")],
            },
        ));
        let new_launch = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::HeaderCreateCommand,
        ));
        let new_folder = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::HeaderCreateFolder,
        ));

        let (_panes, pane) = iced::widget::pane_grid::State::new(());
        let split =
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane {
                    tab_id: 0,
                    pane,
                    axis: iced::widget::pane_grid::Axis::Vertical,
                },
            ));

        assert!(is_header_menu_choice(&open_terminal));
        assert!(is_header_menu_choice(&split));
        assert!(is_header_menu_choice(&launch));
        assert!(is_header_menu_choice(&new_launch));
        assert!(is_header_menu_choice(&new_folder));
    }

    #[test]
    fn given_other_events_when_checked_then_they_are_not_menu_choices() {
        let close_tab =
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab {
                tab_id: 1,
            }));
        let pressed = AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::NodePressed {
                path: vec![String::from("Demo")],
            },
        ));

        assert!(!is_header_menu_choice(&close_tab));
        assert!(!is_header_menu_choice(&pressed));
        assert!(!is_header_menu_choice(&AppEvent::IcedReady));
    }

    #[test]
    fn the_right_panel_button_still_closes_menus_and_inline_edits() {
        let event =
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::ToggleRight));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn hovering_a_files_row_keeps_menus_and_inline_edits() {
        // A row sends the path when the pointer enters it, and no path when
        // the pointer leaves it or the whole tree.
        let paths = [Some(vec![String::from("src")]), None];

        for path in paths {
            let event = AppEvent::Explorer(ExplorerEvent::Intent(
                ExplorerIntent::NodeHovered { path },
            ));

            assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
            assert!(!inline_edit_guard(&event));
        }
    }

    #[test]
    fn moving_over_the_title_bar_keeps_menus_and_inline_edits() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::MoveOnTitleBar));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Allow));
        assert!(!inline_edit_guard(&event));
    }

    #[test]
    fn pressing_a_files_row_still_closes_menus_and_inline_edits() {
        let event = AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            },
        ));

        assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn pressing_the_title_bar_still_cancels_an_inline_edit() {
        let event =
            AppEvent::Chrome(ChromeEvent::Intent(ChromeIntent::PressTitleBar));

        assert!(inline_edit_guard(&event));
    }

    #[test]
    fn a_command_under_an_open_menu_closes_the_menus_and_goes_on() {
        let event = AppEvent::Command(AppCommand::ToggleFocusMode);

        assert!(matches!(
            context_menu_guard(&event),
            MenuGuard::DismissAndRun
        ));
    }

    #[test]
    fn a_command_cancels_an_inline_edit() {
        let event = AppEvent::Command(AppCommand::ToggleFilesPanel);

        assert!(inline_edit_guard(&event));
    }
}
