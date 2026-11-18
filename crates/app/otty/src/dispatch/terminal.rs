//! Delivers the intents of the terminal, with what it reads at the moment
//! of a decision, and reads what it reports: news for the tabs, the modes
//! and the explorer, a failure to tell, and the keyboard and terminal
//! operations of the window.

use std::path::Path;

use iced::Task;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::{
    PaneMachine, TerminalCtx, TerminalIntent, TerminalKind, TerminalOutcome,
    TerminalWorkspace,
};
use otty_app_theme::fonts::FontsConfig;
use otty_ui_term::TerminalView;
use otty_ui_term::settings::Settings;

use super::{Delivery, deliver};
use crate::app::App;
use crate::guards::{self, Overlay};
use crate::message::AppEvent;
use crate::{layout, terminal_settings};

/// The terminal gets the room its panes have, the pointer, the saved
/// Equalize panes setting, the mode and the overlays that are open, and
/// reads what it reports.
impl Delivery for TerminalIntent {
    type Component = TerminalWorkspace;

    fn parts(app: &mut App) -> (&mut TerminalWorkspace, TerminalCtx) {
        let ctx = terminal_ctx(app);

        (&mut app.terminal, ctx)
    }

    fn read(app: &mut App, outcome: TerminalOutcome) -> Task<AppEvent> {
        match outcome {
            // Tabs only show in Terminal mode. A tab tells the modes feature
            // once its terminal runs, so a tab that can not start leaves the
            // mode as it was; the modes feature decides whether it switches.
            TerminalOutcome::TabStarted => deliver(app, ModesIntent::TabShown),
            TerminalOutcome::TitleChanged {
                tab_id,
                title,
                folder,
            } => deliver(
                app,
                TabsIntent::SetTitle {
                    tab_id,
                    title,
                    folder,
                },
            ),
            TerminalOutcome::CloseTab { tab_id } => {
                deliver(app, TabsIntent::CloseTab { tab_id })
            },
            TerminalOutcome::FolderFollowed { cwd } => {
                deliver(app, ExplorerIntent::SyncRoot { cwd })
            },
            TerminalOutcome::FocusTerminal(id) => TerminalView::focus(id),
            TerminalOutcome::TakeKeyboard => super::take_keyboard(),
            TerminalOutcome::RunBlockCommand { terminal, command } => {
                TerminalView::command(terminal, command)
            },
            TerminalOutcome::Failed(text) => {
                deliver(app, NotificationsIntent::Push(text))
            },
        }
    }
}

/// Start the user's shell in the new terminal tab `tab_id`, with the
/// settings of the theme and the session built at start, in `folder` or
/// else in the folder the session starts in.
pub(crate) fn open_shell_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
    folder: Option<&Path>,
) -> Task<AppEvent> {
    let session = app.shell_session.session(folder);
    let settings = terminal_settings::for_session(&base_settings(app), session);

    open_tab(app, tab_id, title, settings, TerminalKind::Shell)
}

/// Start the command of the new command tab `tab_id` with the settings it
/// came with.
pub(crate) fn open_command_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
    settings: Settings,
    exit_behavior: ExitBehavior,
) -> Task<AppEvent> {
    let kind = TerminalKind::Command { exit_behavior };

    open_tab(app, tab_id, title, settings, kind)
}

/// The settings new terminals start with: the colors of the current theme
/// and the terminal font, before a session is set.
pub(crate) fn base_settings(app: &App) -> Settings {
    terminal_settings::base(
        app.theme_manager.current(),
        &FontsConfig::default(),
    )
}

/// Give the terminal the new tab, which is the active tab, named after the
/// machine the tabs say it runs on, with the home folder read at start.
fn open_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
    settings: Settings,
    kind: TerminalKind,
) -> Task<AppEvent> {
    let machine = PaneMachine {
        name: app.tabs.active_machine_name().to_string(),
        target: app.tabs.active_machine_target(),
        home: app.home.clone(),
    };

    deliver(
        app,
        TerminalIntent::OpenTab {
            tab_id,
            title,
            settings: Box::new(settings),
            kind,
            machine,
        },
    )
}

/// What the terminal reads at the moment of a decision.
fn terminal_ctx(app: &App) -> TerminalCtx {
    let top = guards::top_overlay(app);

    TerminalCtx {
        pane_grid_size: layout::pane_grid_size(
            app.screen_size,
            app.sidebar.side_panels_ratio(),
        ),
        screen_size: app.screen_size,
        cursor: app.sidebar.cursor(),
        equalize_panes: app.settings.saved().equalize_panes(),
        is_terminal_mode: app.modes.active() == Mode::Terminal,
        is_overlay_open: top.is_some(),
        is_other_overlay_open: top
            .is_some_and(|overlay| overlay != Overlay::TerminalContextMenu),
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_settings::{SettingsIntent, SettingsLoad, SettingsLoadStatus};
    use otty_app_testing::blocks::{command_block, finished, prompt_block};
    use otty_app_testing::terminal::ScriptedTerminals;
    use otty_libterm::surface::{BlockKey, BlockSnapshot, Scroll};
    use otty_ui_term::{SnapshotOwned, TerminalRequest};

    use super::*;
    use crate::dispatch;
    use crate::testing::running::RunningApp;
    use crate::testing::window::{
        app, app_with_scripted_terminals, open_palette, open_pane_menu,
        open_settings, open_shell_tab, test_settings,
    };
    use crate::view::view;

    /// Owner, Q4 of the old app: a split shares the room by the saved
    /// Equalize panes setting, not by a change in the open Settings dialog
    /// that is not saved yet.
    #[test]
    fn the_terminal_reads_the_saved_equalize_setting_not_an_unsaved_one() {
        let mut app = app();
        let mut saved = test_settings();
        saved.set_equalize_panes(true);
        let _ = deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                saved,
                SettingsLoadStatus::Loaded,
            )),
        );
        open_settings(&mut app);
        let _ = deliver(&mut app, SettingsIntent::EqualizePanesToggled(false));

        assert!(terminal_ctx(&app).equalize_panes);
    }

    /// The pane grid is the main area without the shown side panels; the
    /// pane menu is kept inside the whole main area and opens at the
    /// pointer.
    #[test]
    fn the_pane_grid_is_the_main_area_without_the_side_panels() {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(Size::new(
                1000.0, 700.0,
            ))),
        );

        let ctx = terminal_ctx(&app);

        assert_eq!(ctx.screen_size, app.screen_size);
        assert_eq!(
            ctx.pane_grid_size,
            layout::pane_grid_size(
                app.screen_size,
                app.sidebar.side_panels_ratio()
            )
        );
        assert!(ctx.pane_grid_size.width < ctx.screen_size.width);
        assert_eq!(ctx.cursor, app.sidebar.cursor());
    }

    /// The pane menu alone is the terminal's own overlay: a pane that
    /// closes closes it too. Another overlay over it stays.
    #[test]
    fn the_terminal_tells_its_own_menu_from_the_other_overlays() {
        let mut app = app();
        let plain = terminal_ctx(&app);
        open_pane_menu(&mut app);
        let own_menu = terminal_ctx(&app);
        open_palette(&mut app);
        let other = terminal_ctx(&app);

        assert!(!plain.is_overlay_open);
        assert!(plain.is_terminal_mode);
        assert!(own_menu.is_overlay_open && !own_menu.is_other_overlay_open);
        assert!(other.is_overlay_open && other.is_other_overlay_open);
    }

    /// Owner: a split with no room gives a toast. The window here is
    /// narrower than two panes of the minimum width.
    #[test]
    fn a_split_with_no_room_tells_the_user() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(Size::new(
                200.0, 600.0,
            ))),
        );

        let _ = deliver(
            &mut app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        );

        assert_eq!(app.terminal.pane_count(), Some(1));
        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(view(&app));
        assert!(ui.find(i18n::t(Key::NoRoomForPane)).is_ok());
    }

    /// A split that fits shows both panes and tells nothing.
    #[test]
    fn a_split_that_fits_shows_both_panes() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);

        let _ = deliver(
            &mut app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        );

        assert_eq!(app.terminal.pane_count(), Some(2));
        assert_eq!(app.notifications.unseen_count(), 0);
    }

    /// The pane menu opens over the main area with its items.
    #[test]
    fn the_open_pane_menu_shows_over_the_window() {
        let mut app = app();

        open_pane_menu(&mut app);

        let mut ui = simulator(view(&app));
        assert!(ui.find(i18n::t(Key::CopySelection)).is_ok());
        assert!(ui.find(i18n::t(Key::ClosePane)).is_ok());
    }

    /// The shell of the only pane ends: its tab closes in the same update.
    #[test]
    fn a_shell_that_ends_closes_its_tab() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);
        let terminal_id =
            app.terminal.focused_terminal_id().expect("the terminal");

        let _ = deliver(
            &mut app,
            TerminalIntent::Widget(otty_ui_term::Event::Shutdown {
                id: terminal_id,
                exit_status: std::process::ExitStatus::default(),
            }),
        );

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.terminal.pane_count(), None);
    }

    /// The text of `ls` in the snapshots of [`ls_at_the_prompt`]: its
    /// command line, then what it printed.
    const LS_TEXT: &str = "$ ls\nCargo.toml\nREADME.md";
    /// The key of the `ls` block.
    const LS: BlockKey = BlockKey::new(1);

    /// A terminal that waits at the prompt under the `ls` block, which
    /// ended well after 0.1 s and printed two names; the block is folded
    /// when `is_folded` is set. The status of the block shows `0.1 s`.
    fn ls_at_the_prompt(is_folded: bool) -> SnapshotOwned {
        let mut ls = BlockSnapshot {
            key: LS,
            line_count: 6,
            cached_text: Some(LS_TEXT.into()),
            command_line_count: 1,
            is_foldable: true,
            hidden_line_count: is_folded.then_some(2),
            ..finished(command_block("ls", 1_000), Some(0), 100)
        };
        ls.meta.cmd = Some(String::from("ls"));
        let prompt = BlockSnapshot {
            start_line: 6,
            ..prompt_block()
        };
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![ls, prompt];

        snapshot
    }

    /// The app with one terminal tab whose terminal shows `snapshot`, run
    /// in a window, with the terminals of the test and the id of the
    /// terminal.
    fn running_terminal(
        snapshot: SnapshotOwned,
    ) -> (RunningApp, ScriptedTerminals, u64) {
        let (app, terminals) = app_with_scripted_terminals();
        let mut window = RunningApp::new(app);
        window.deliver(AppEvent::Tabs(TabsIntent::OpenTerminalTab {
            folder: None,
        }));
        let terminal_id = window
            .app()
            .terminal
            .focused_terminal_id()
            .expect("the tab has a terminal");

        terminals.report(terminal_id, snapshot);
        window.deliver(AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::ContentSync { id: terminal_id },
        )));

        (window, terminals, terminal_id)
    }

    /// Right-click the output of `ls` in the window and press `item` in
    /// the menu that opens.
    fn press_in_the_menu_of_ls(window: &mut RunningApp, item: &str) {
        let status =
            window.find("0.1 s").expect("the status of ls should show");
        // Two lines under the status line: the output of the block.
        window.move_to(Point::new(status.x, status.y + 2.5 * status.height));
        window.click(iced::mouse::Button::Right);

        window.click_text(item);
    }

    /// Whether the terminal asked its program to fold or unfold `ls`.
    fn asked_to_fold_ls(requests: &[TerminalRequest]) -> bool {
        requests.iter().any(|request| {
            matches!(
                request,
                TerminalRequest::ToggleBlockFold { block_key: LS }
            )
        })
    }

    /// What the terminal wrote to its shell, write after write.
    fn written(requests: &[TerminalRequest]) -> Vec<Vec<u8>> {
        requests
            .iter()
            .filter_map(|request| match request {
                TerminalRequest::WriteBytes(data) => Some(data.clone()),
                _ => None,
            })
            .collect()
    }

    /// Issue 43: Tab in a focused terminal goes to its shell, as the tab
    /// character; no field takes it. Shift+Tab goes as the back-tab
    /// sequence.
    #[test]
    fn tab_in_a_focused_terminal_goes_to_its_shell() {
        let (mut window, terminals, terminal_id) =
            running_terminal(SnapshotOwned::default());

        window.tap_tab(iced::keyboard::Modifiers::empty());
        window.tap_tab(iced::keyboard::Modifiers::SHIFT);

        let requests = terminals.requests(terminal_id);
        assert_eq!(
            written(&requests),
            [b"\t".to_vec(), b"\x1b[Z".to_vec()],
            "got {requests:?}"
        );
    }

    /// Owner, issue 11: `Collapse` in the menu of a block folds it, as the
    /// arrow does, from the right-click to the request to the program.
    #[test]
    fn collapse_in_the_menu_of_a_block_asks_its_terminal_to_fold_it() {
        let (mut window, terminals, terminal_id) =
            running_terminal(ls_at_the_prompt(false));

        press_in_the_menu_of_ls(&mut window, i18n::t(Key::Collapse));

        let requests = terminals.requests(terminal_id);
        assert!(asked_to_fold_ls(&requests), "got {requests:?}");
        assert!(!window.app().terminal.has_context_menu());
    }

    /// Owner, issue 11: `Expand` in the menu of a folded block unfolds it.
    #[test]
    fn expand_in_the_menu_of_a_folded_block_asks_its_terminal_to_unfold_it() {
        let (mut window, terminals, terminal_id) =
            running_terminal(ls_at_the_prompt(true));

        press_in_the_menu_of_ls(&mut window, i18n::t(Key::Expand));

        let requests = terminals.requests(terminal_id);
        assert!(asked_to_fold_ls(&requests), "got {requests:?}");
    }

    /// Owner, issue 11: `Run again` types the command of the block at the
    /// prompt and presses Enter; the pane scrolls to the bottom and keeps
    /// the keyboard, so what the user types next reaches the shell.
    #[test]
    fn run_again_in_the_menu_of_a_block_runs_its_command_in_the_shell() {
        let (mut window, terminals, terminal_id) =
            running_terminal(ls_at_the_prompt(false));

        press_in_the_menu_of_ls(&mut window, i18n::t(Key::RunAgain));
        window.type_text("x");

        let requests = terminals.requests(terminal_id);
        assert_eq!(
            written(&requests),
            [b"\x05\x15ls\r".to_vec(), b"x".to_vec()],
            "got {requests:?}"
        );
        let run_at = requests
            .iter()
            .position(|request| {
                matches!(
                    request,
                    TerminalRequest::WriteBytes(data)
                        if data == b"\x05\x15ls\r"
                )
            })
            .expect("the command is written");
        assert!(
            matches!(
                requests.get(run_at + 1),
                Some(TerminalRequest::ScrollDisplay(Scroll::Bottom))
            ),
            "got {requests:?}"
        );
    }

    /// A screen of 10 rows that printed 100 lines, with `ls` at the top
    /// (its status says `0.1 s`) and a program that reads the mouse in the
    /// normal screen, as `fzf --height 40%` does.
    fn long_history_read_by_a_program() -> SnapshotOwned {
        use otty_libterm::escape::NamedPrivateMode;
        use otty_libterm::surface::{
            Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
        };

        let size = otty_libterm::TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 80,
            rows: 10,
        };
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        for _ in 1..100 {
            surface.line_feed();
        }
        for mode in [
            NamedPrivateMode::ReportMouseClicks,
            NamedPrivateMode::ReportCellMouseMotion,
            NamedPrivateMode::SgrMouse,
        ] {
            surface.set_private_mode(mode.into());
        }
        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks = ls_at_the_prompt(false).blocks;

        snapshot
    }

    /// Owner, issue 12: the scroll mark is dragged with the mouse, also
    /// past the edge of the window. The drag scrolls the history of OTTY
    /// and sends nothing to a program that reads the mouse.
    #[test]
    fn the_scroll_mark_dragged_out_of_the_window_scrolls_the_history_and_tells_the_program_nothing()
     {
        let (mut window, terminals, terminal_id) =
            running_terminal(long_history_read_by_a_program());
        let status =
            window.find("0.1 s").expect("the status of ls should show");
        let pane = window
            .find_focusable_at(status.center())
            .expect("the terminal is under the status of ls");
        // The view is at the bottom of the history, so the mark is at the
        // bottom of the pane; the strip it runs in is the right inset.
        let strip_x = pane.x + pane.width - 4.0;

        window.move_to(Point::new(strip_x, pane.y + pane.height - 10.0));
        window.press(iced::mouse::Button::Left);
        window.move_to(Point::new(strip_x, pane.y + pane.height / 2.0));
        window.move_to(Point::new(pane.x + pane.width + 400.0, -60.0));
        window.release(iced::mouse::Button::Left);
        window.move_to(Point::new(pane.x + 100.0, pane.y + 100.0));

        let requests = terminals.requests(terminal_id);
        let offsets: Vec<usize> = requests
            .iter()
            .filter_map(|request| match request {
                TerminalRequest::ScrollDisplay(Scroll::Offset(offset)) => {
                    Some(*offset)
                },
                _ => None,
            })
            .collect();
        // 100 lines, 10 on screen: the top of the history is 90 lines up.
        // Half way down the pane the mark stands for a place in the middle;
        // above the window, at the top of its way, for the top of the
        // history and the step past it.
        assert_eq!(offsets.len(), 1, "got {requests:?}");
        assert!(offsets[0] > 0 && offsets[0] < 90, "got {offsets:?}");
        let to_the_top = requests
            .iter()
            .filter(|request| {
                matches!(request, TerminalRequest::ScrollDisplay(Scroll::Top))
            })
            .count();
        assert_eq!(to_the_top, 1, "got {requests:?}");
        assert!(written(&requests).is_empty(), "got {requests:?}");
    }

    /// The control case of issue 12: with the button up, a press in the
    /// text of a program that reads the mouse goes to the program.
    #[test]
    fn a_press_in_the_text_of_a_program_that_reads_the_mouse_is_reported_to_it()
    {
        let (mut window, terminals, terminal_id) =
            running_terminal(long_history_read_by_a_program());
        let status =
            window.find("0.1 s").expect("the status of ls should show");
        let pane = window
            .find_focusable_at(status.center())
            .expect("the terminal is under the status of ls");

        window.move_to(Point::new(pane.x + 100.0, pane.y + 100.0));
        window.click(iced::mouse::Button::Left);

        let requests = terminals.requests(terminal_id);
        assert_eq!(written(&requests).len(), 2, "got {requests:?}");
    }

    /// The control case of issue 11: `Copy output` copies what `ls`
    /// printed, along the same path as the items above.
    #[test]
    fn copy_output_in_the_menu_of_a_block_copies_what_it_printed() {
        let (mut window, _terminals, _terminal_id) =
            running_terminal(ls_at_the_prompt(false));

        press_in_the_menu_of_ls(&mut window, i18n::t(Key::CopyContent));

        assert_eq!(window.clipboard_text(), Some("Cargo.toml\nREADME.md"));
    }

    /// `Paste` in the pane menu writes the clipboard to the shell, as typed
    /// text, and the pane keeps the keyboard: what the user types next
    /// reaches the shell too.
    #[test]
    fn paste_in_the_pane_menu_writes_the_clipboard_to_the_shell() {
        let (mut window, terminals, terminal_id) =
            running_terminal(ls_at_the_prompt(false));
        press_in_the_menu_of_ls(&mut window, i18n::t(Key::CopyContent));

        press_in_the_menu_of_ls(&mut window, i18n::t(Key::Paste));
        window.type_text("x");

        let requests = terminals.requests(terminal_id);
        assert_eq!(
            written(&requests),
            [b"Cargo.toml\rREADME.md".to_vec(), b"x".to_vec()],
            "got {requests:?}"
        );
        assert!(!window.app().terminal.has_context_menu());
    }
}
