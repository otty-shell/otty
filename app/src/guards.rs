//! What input from the user does while a menu is open or an inline edit runs,
//! and which overlay lies on top of the window. The input checks only see
//! input: what follows from it, and what arrives on its own, skips them (see
//! `update_internal` in `update.rs`), so it can not get lost.

use crate::app::App;
use crate::domain::mode::Mode;
use crate::router::AppEvent;
use crate::widgets::chrome::event::{ChromeEvent, ChromeIntent};
use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
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
    /// Return `true` for a menu: the palette, the All modes menu, the header
    /// menus, the Containers row menu and the context menus. A click
    /// outside a menu closes it, and a shortcut closes it and runs.
    pub(crate) fn is_menu(self) -> bool {
        matches!(
            self,
            Overlay::Palette
                | Overlay::ModesMenu
                | Overlay::HeaderMenu
                | Overlay::ContainersRowMenu
                | Overlay::TerminalContextMenu
                | Overlay::LaunchContextMenu
        )
    }

    /// Return `true` for a dialog: the SSH prompt and the Containers create
    /// and confirm dialogs. A dialog waits for its answer.
    pub(crate) fn is_dialog(self) -> bool {
        matches!(
            self,
            Overlay::SecretPrompt
                | Overlay::ContainersCreate
                | Overlay::ContainersConfirm
        )
    }
}

/// Determines how the event loop should treat input from the user when a
/// context menu is open.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MenuGuard {
    /// Let the event pass through to normal dispatch.
    Allow,
    /// Silently drop the event without closing the menu.
    Ignore,
    /// Close all context menus; the event itself does nothing else.
    Dismiss,
    /// Close all context menus, then dispatch the event.
    DismissAndRun,
}

/// Classify input from the user when at least one context menu is open.
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
        // The first press on a panel button or the palette button only closes
        // the menu; opening settings closes it and goes on.
        AppEvent::Chrome(ChromeEvent::Intent(event)) => {
            use ChromeIntent as E;
            match event {
                E::ToggleLeftPanel
                | E::ToggleFilesPanel
                | E::ToggleFocusMode
                | E::OpenCommandPalette => Dismiss,
                E::OpenSettings => DismissAndRun,
                _ => Allow,
            }
        },
        AppEvent::Explorer(ExplorerEvent::Intent(
            ExplorerIntent::NodeHovered { .. },
        )) => Allow,
        AppEvent::Tabs(_) => DismissAndRun,
        // The All modes menu is one of the menus: its rows, pin toggles and
        // dismiss layer send these. Choosing a mode closes every menu.
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(_))) => {
            DismissAndRun
        },
        AppEvent::Modes(_) => Allow,
        AppEvent::ResizeWindow(_) => Allow,
        // A shortcut, whichever key route it took, closes the menus and
        // runs. Other keys no widget took do nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// Return the overlay on top of the window, if one is open.
///
/// The order is fixed, top first: the SSH prompt, the palette, the All modes
/// menu, the header menus, then the overlays of the active mode. Those of the
/// other mode are not on screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
    let widgets = &app.widgets;

    if widgets.quick_launch.has_secret_prompt() {
        return Some(Overlay::SecretPrompt);
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
/// took. The app closes the All modes menu with it.
pub(crate) fn is_escape_press(event: &AppEvent) -> bool {
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event, Key};

    matches!(
        event,
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
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

    use super::{
        MenuGuard, Overlay, context_menu_guard, inline_edit_guard,
        is_escape_press, is_header_menu_choice, is_terminal_input,
        is_terminal_input_kept_out, top_overlay,
    };
    use crate::app::App;
    use crate::commands::AppCommand;
    use crate::domain::mode::Mode;
    use crate::router::AppEvent;
    use crate::shortcuts::AppShortcut;
    use crate::testing::overlays::{
        open_confirm, open_create_dialog, open_header_menu, open_launch_menu,
        open_modes_menu, open_palette, open_pane_menu, open_row_menu,
        open_secret_prompt, start_inline_edit,
    };
    use crate::testing::window::{app_in, containers_app_with_lists};
    use crate::widgets::chrome::event::{ChromeEvent, ChromeIntent};
    use crate::widgets::containers::actions::event::ActionsIntent;
    use crate::widgets::containers::actions::types::RowAction;
    use crate::widgets::containers::event::{
        ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::types::InspectorTab;
    use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
    use crate::widgets::modes::event::{ModesEvent, ModesIntent};
    use crate::widgets::quick_launch::event::{
        QuickLaunchEvent, QuickLaunchIntent,
    };
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
    const TERMINAL_OVERLAYS: [(Opener, Overlay); 7] = [
        (open_secret_prompt, Overlay::SecretPrompt),
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
    const MENUS: [Overlay; 6] = [
        Overlay::Palette,
        Overlay::ModesMenu,
        Overlay::HeaderMenu,
        Overlay::ContainersRowMenu,
        Overlay::TerminalContextMenu,
        Overlay::LaunchContextMenu,
    ];

    /// The dialogs: they wait for their answer.
    const DIALOGS: [Overlay; 3] = [
        Overlay::SecretPrompt,
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
    fn nothing_is_on_top_of_a_plain_window() {
        let terminal = app_in(Mode::Terminal);
        let containers = containers_app_with_lists();

        assert_eq!(top_overlay(&terminal), None);
        assert_eq!(top_overlay(&containers), None);
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
        let menus: [Opener; 5] = [
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
            containers(ContainersIntent::Actions(ActionsIntent::Act(
                RowAction::Stop,
            ))),
            containers(ContainersIntent::Actions(ActionsIntent::Prune)),
        ];

        for event in clicks {
            assert!(matches!(context_menu_guard(&event), MenuGuard::Dismiss));
            assert!(inline_edit_guard(&event));
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
