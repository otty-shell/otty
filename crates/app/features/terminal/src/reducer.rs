use std::collections::HashMap;
use std::rc::Rc;

use iced::widget::pane_grid;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_term::BlockCommand;
use otty_ui_term::settings::Settings;

use super::blocks::{self, BlockStep};
use super::ctx::TerminalCtx;
use super::event::{TerminalIntent, TerminalOutcome};
use super::state::{StateCommand, TerminalTabState, TerminalWorkspaceState};
use super::types::{PaneMachine, StartTerminal, TerminalEvent, TerminalKind};

/// A tab to open, as [`TerminalIntent::OpenTab`] gives it.
struct NewTab {
    tab_id: u64,
    title: String,
    settings: Settings,
    kind: TerminalKind,
    machine: PaneMachine,
}

/// A pane to split, as [`TerminalIntent::SplitPane`] and
/// [`TerminalIntent::SplitFocusedPaneWith`] give it.
struct Split {
    tab_id: u64,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
    /// What the new pane starts in place of the tab's own settings.
    settings: Option<Settings>,
}

/// Reduce a terminal intent into state updates and the outcomes the binary
/// reads, in the order they happened.
pub(super) fn reduce(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    start_terminal: &StartTerminal,
    intent: TerminalIntent,
    ctx: &TerminalCtx,
) -> Vec<TerminalOutcome> {
    use TerminalIntent::*;

    let follows_folder = follows_folder(&intent);

    let mut outcomes = match intent {
        OpenTab {
            tab_id,
            title,
            settings,
            kind,
            machine,
        } => reduce_open_tab(
            state,
            terminal_to_tab,
            next_terminal_id,
            start_terminal,
            ctx,
            NewTab {
                tab_id,
                title,
                settings: *settings,
                kind,
                machine,
            },
        ),
        TabActivated { tab_id } => {
            state.set_active_tab_id(Some(tab_id));

            reduce_tab_in_front(state, ctx)
        },
        TabClosed {
            tab_id,
            new_active_id,
        } => {
            let _ = state.remove_tab(tab_id);
            terminal_to_tab.retain(|_, mapped_tab| *mapped_tab != tab_id);
            state.set_active_tab_id(new_active_id);

            reduce_tab_in_front(state, ctx)
        },
        Widget(event) => {
            reduce_widget_event(state, terminal_to_tab, event, ctx)
        },
        PaneClicked { tab_id, pane } => {
            with_terminal_tab(state, tab_id, |tab| tab.focus_pane(pane))
        },
        PaneResized { tab_id, event } => {
            with_terminal_tab(state, tab_id, |tab| {
                tab.resize(event);
                StateCommand::None
            })
        },
        PaneGridCursorMoved { tab_id, position } => {
            with_terminal_tab(state, tab_id, |tab| {
                tab.update_grid_cursor(position)
            })
        },
        OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        } => with_terminal_tab(state, tab_id, |tab| {
            tab.open_context_menu(
                pane,
                terminal_id,
                ctx.cursor,
                ctx.screen_size,
            )
        }),
        CloseContextMenu { tab_id } => {
            with_terminal_tab(state, tab_id, |tab| tab.close_context_menu())
        },
        SplitPane { tab_id, pane, axis } => reduce_split_pane(
            state,
            terminal_to_tab,
            next_terminal_id,
            Split {
                tab_id,
                pane,
                axis,
                settings: None,
            },
            ctx.equalize_panes,
        ),
        SplitFocusedPane { axis } => reduce_split_focused_pane(
            state,
            terminal_to_tab,
            next_terminal_id,
            ctx,
            axis,
            None,
        ),
        SplitFocusedPaneWith { axis, settings } => {
            if !state.active_tab().is_some_and(|tab| tab.runs_locally()) {
                return Vec::new();
            }

            reduce_split_focused_pane(
                state,
                terminal_to_tab,
                next_terminal_id,
                ctx,
                axis,
                Some(*settings),
            )
        },
        ClosePane { tab_id, pane } => {
            // Close pane in the pane menu closes the menu with the pane, so
            // the pane left takes the keyboard although the menu was open.
            let equalize_panes = ctx.equalize_panes;
            let outcomes = with_terminal_tab(state, tab_id, |tab| {
                tab.close_pane(pane, equalize_panes, true)
            });
            reindex_terminal_tabs(state, terminal_to_tab);

            outcomes
        },
        RunBlockCommand {
            tab_id,
            terminal_id,
            command,
        } => reduce_block_command(state, tab_id, terminal_id, command),
        SelectPreviousBlock => reduce_block_step(state, BlockStep::Previous),
        SelectNextBlock => reduce_block_step(state, BlockStep::Next),
        ApplyTheme { palette } => {
            for (_, tab) in state.tabs_mut() {
                tab.apply_theme(*palette.clone());
            }

            Vec::new()
        },
        CloseAllContextMenus => {
            let mut outcomes = Vec::new();
            for (_, tab) in state.tabs_mut() {
                if tab.context_menu().is_some() {
                    push_command(tab.close_context_menu(), &mut outcomes);
                }
            }

            outcomes
        },
        FocusActive => focus_active(state, ctx),
        ModeChanged => reduce_mode_changed(state, ctx),
        SyncPaneGridSize => {
            for (_, tab) in state.tabs_mut() {
                tab.set_grid_size(ctx.pane_grid_size);
            }

            Vec::new()
        },
        // The binary reads the clock when it builds the view: nothing to
        // change.
        BlockClockTick => Vec::new(),
    };

    if follows_folder {
        outcomes.extend(folder_followed(state));
    }

    outcomes
}

/// Whether `intent` may change the tab in front, its focused pane or the
/// folder its terminal is in, so the Files section should follow the
/// active terminal again. It does not depend on what the reducer does with
/// the intent: an intent for a tab or terminal that is gone still follows
/// the folder of the tab in front.
fn follows_folder(intent: &TerminalIntent) -> bool {
    use TerminalIntent::*;

    matches!(
        intent,
        OpenTab { .. }
            | TabActivated { .. }
            | TabClosed { .. }
            | PaneClicked { .. }
            | SplitPane { .. }
            | SplitFocusedPane { .. }
            | SplitFocusedPaneWith { .. }
            | ClosePane { .. }
            | Widget(otty_ui_term::Event::ContentSync { .. })
    )
}

/// The folder of the tab in front, if it runs on this computer and its
/// shell reported one.
fn folder_followed(state: &TerminalWorkspaceState) -> Option<TerminalOutcome> {
    let cwd = state.active_local_cwd()?;

    Some(TerminalOutcome::FolderFollowed { cwd })
}

/// Start the terminal of a new tab, which comes to the front.
fn reduce_open_tab(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    start_terminal: &StartTerminal,
    ctx: &TerminalCtx,
    new_tab: NewTab,
) -> Vec<TerminalOutcome> {
    let NewTab {
        tab_id,
        title,
        settings,
        kind,
        machine,
    } = new_tab;
    let terminal_id = *next_terminal_id;
    *next_terminal_id += 1;
    state.set_active_tab_id(Some(tab_id));

    let failed_tab_title = title.clone();
    let mut terminal_tab = match TerminalTabState::new(
        tab_id,
        title,
        terminal_id,
        settings,
        kind,
        machine,
        Rc::clone(start_terminal),
    ) {
        Ok(tab) => tab,
        // The tab closes, and the user learns why: a launch by its name,
        // a shell as a terminal that did not open. No started tab is
        // reported, so the mode stays as it was.
        Err(err) => {
            log::warn!("failed to create terminal tab: {err}");

            let text = match kind {
                TerminalKind::Command { .. } => {
                    i18n::launch_failed(&failed_tab_title, &format!("{err}"))
                },
                _ => i18n::with_error(
                    Key::TplTerminalInitFailed,
                    &format!("{err}"),
                ),
            };

            return vec![
                TerminalOutcome::CloseTab { tab_id },
                TerminalOutcome::Failed(text),
            ];
        },
    };

    terminal_tab.set_grid_size(ctx.pane_grid_size);
    for t_id in terminal_tab.terminals().keys().copied() {
        terminal_to_tab.insert(t_id, tab_id);
    }
    let title_changed = title_changed(tab_id, &terminal_tab);
    state.insert_tab(tab_id, terminal_tab);

    // The tab is really there now, so the app may show Terminal mode. The
    // new pane takes the keyboard, unless an open overlay keeps it.
    let mut outcomes = vec![title_changed, TerminalOutcome::TabStarted];
    outcomes.extend(focus_active(state, ctx));

    outcomes
}

/// The tab in front changed: its focused pane takes the keyboard, unless an
/// overlay keeps it, and its panes draw the block selection it keeps. A tab
/// in front that is no terminal tab, or none, moves nothing.
fn reduce_tab_in_front(
    state: &TerminalWorkspaceState,
    ctx: &TerminalCtx,
) -> Vec<TerminalOutcome> {
    let mut outcomes = focus_active(state, ctx);
    outcomes.extend(sync_selection(state));

    outcomes
}

/// Give the focused pane of the tab in front the keyboard. An open overlay
/// keeps it instead, and outside Terminal mode no pane is on screen.
fn focus_active(
    state: &TerminalWorkspaceState,
    ctx: &TerminalCtx,
) -> Vec<TerminalOutcome> {
    if ctx.is_overlay_open || !ctx.is_terminal_mode {
        return Vec::new();
    }
    let Some(entry) = state
        .active_tab()
        .and_then(|tab| tab.focused_terminal_entry())
    else {
        return Vec::new();
    };

    vec![TerminalOutcome::FocusTerminal(
        entry.terminal().widget_id().clone(),
    )]
}

/// Back in Terminal mode the panes take the grid size the window has now,
/// and the active pane the keyboard. Leaving it changes nothing.
fn reduce_mode_changed(
    state: &mut TerminalWorkspaceState,
    ctx: &TerminalCtx,
) -> Vec<TerminalOutcome> {
    if !ctx.is_terminal_mode {
        return Vec::new();
    }

    for (_, tab) in state.tabs_mut() {
        tab.set_grid_size(ctx.pane_grid_size);
    }

    focus_active(state, ctx)
}

fn reduce_widget_event(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    event: TerminalEvent,
    ctx: &TerminalCtx,
) -> Vec<TerminalOutcome> {
    let terminal_id = *event.terminal_id();
    let Some(tab_id) = terminal_to_tab.get(&terminal_id).copied() else {
        return Vec::new();
    };

    // A pane closes when its process exits, or on a key press once it
    // has exited and was kept open.
    let may_close_pane = match &event {
        otty_ui_term::Event::Shutdown { .. } => true,
        otty_ui_term::Event::Write { .. } => state
            .tab(tab_id)
            .is_some_and(|tab| tab.has_exited(terminal_id)),
        _ => false,
    };
    // When a pane closes, the pane left takes the keyboard only in the tab
    // on screen, and only when no overlay stays open. The pane's own menu
    // closes with the pane; another overlay, or the menu of another pane,
    // stays open and keeps the keyboard. Otherwise the pane left only
    // becomes the tab's focused pane, and `FocusActive` gives it the
    // keyboard later.
    let equalize_panes = ctx.equalize_panes;
    let own_menu_is_open = state
        .tab(tab_id)
        .and_then(|tab| tab.context_menu())
        .is_some_and(|menu| menu.terminal_id() == terminal_id);
    let no_overlay_stays_open = !ctx.is_overlay_open
        || (own_menu_is_open && !ctx.is_other_overlay_open);
    let focus_terminal_widget =
        state.active_tab_id() == Some(tab_id) && no_overlay_stays_open;

    let mut outcomes = update_block_selection(state, tab_id, &event);
    outcomes.extend(with_terminal_tab(state, tab_id, |tab| {
        tab.handle_terminal_event(event, equalize_panes, focus_terminal_widget)
    }));

    if may_close_pane {
        reindex_terminal_tabs(state, terminal_to_tab);
    }

    outcomes
}

/// Split the focused pane of the tab in front; `settings` starts in the new
/// pane in place of the tab's own when given. Nothing happens when the tab
/// in front is not a terminal tab or has no focused pane.
fn reduce_split_focused_pane(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    ctx: &TerminalCtx,
    axis: pane_grid::Axis,
    settings: Option<Settings>,
) -> Vec<TerminalOutcome> {
    let Some(tab_id) = state.active_tab_id() else {
        return Vec::new();
    };
    let Some(pane) = state.tab(tab_id).and_then(|tab| tab.focus()) else {
        return Vec::new();
    };

    reduce_split_pane(
        state,
        terminal_to_tab,
        next_terminal_id,
        Split {
            tab_id,
            pane,
            axis,
            settings,
        },
        ctx.equalize_panes,
    )
}

fn reduce_split_pane(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    split: Split,
    equalize_panes: bool,
) -> Vec<TerminalOutcome> {
    let Split {
        tab_id,
        pane,
        axis,
        settings,
    } = split;
    let terminal_id = *next_terminal_id;
    *next_terminal_id += 1;

    let outcomes =
        with_terminal_tab(state, tab_id, move |tab| match settings {
            Some(settings) => tab.split_pane_with(
                pane,
                axis,
                terminal_id,
                equalize_panes,
                settings,
            ),
            None => tab.split_pane(pane, axis, terminal_id, equalize_panes),
        });

    if state
        .tab(tab_id)
        .is_some_and(|tab| tab.contains_terminal(terminal_id))
    {
        terminal_to_tab.insert(terminal_id, tab_id);
    }

    outcomes
}

/// Run `command` on the terminal `terminal_id` of the tab `tab_id`: copy
/// its text selection or paste into it, or on one of its blocks fold it,
/// run it again, copy a part of it, or bring it into view. The terminal
/// knows whether the block can do it; nothing runs when the terminal is
/// gone. Then an open pane menu of the tab closes, as after any of its
/// items: the block selection clears and the focused pane takes the
/// keyboard back. A command names its block by key, so the cleared
/// selection does not change what it acts on. The paste writes to the
/// terminal by its id, so it reaches the terminal of the menu whichever
/// pane takes the keyboard.
fn reduce_block_command(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    terminal_id: u64,
    command: BlockCommand,
) -> Vec<TerminalOutcome> {
    let Some(widget_id) = state
        .tab(tab_id)
        .and_then(|tab| tab.terminals().get(&terminal_id))
        .map(|entry| entry.terminal().widget_id().clone())
    else {
        return Vec::new();
    };

    let mut outcomes = vec![TerminalOutcome::RunBlockCommand {
        terminal: widget_id,
        command,
    }];
    outcomes.extend(with_terminal_tab(state, tab_id, |tab| {
        tab.close_context_menu()
    }));

    outcomes
}

/// Move the block selection of the focused pane of the tab in front by
/// `step` and bring the block into sight there. The tab keeps one
/// selection, so the other panes clear theirs. Nothing happens while the
/// pane shows a full-screen program (it has no blocks, and the keys reach
/// the program), or when the selection would stay.
fn reduce_block_step(
    state: &mut TerminalWorkspaceState,
    step: BlockStep,
) -> Vec<TerminalOutcome> {
    let Some(tab_id) = state.active_tab_id() else {
        return Vec::new();
    };
    let Some((terminal_id, widget_id, block_key)) =
        state.tab(tab_id).and_then(|tab| {
            let entry = tab.focused_terminal_entry()?;
            let terminal = entry.terminal();
            let snapshot = terminal.snapshot();
            let view = snapshot.view();
            if view.is_full_screen() {
                return None;
            }

            let selected = tab.selected_block_key_in(terminal.id);
            let block_key =
                blocks::block_after_step(view.blocks(), selected, step)?;

            Some((terminal.id, terminal.widget_id().clone(), block_key))
        })
    else {
        return Vec::new();
    };

    if let Some(tab) = state.tab_mut(tab_id) {
        tab.set_selected_block(terminal_id, block_key);
    }

    let mut outcomes = sync_selection(state);
    outcomes.push(TerminalOutcome::RunBlockCommand {
        terminal: widget_id,
        command: BlockCommand::ScrollTo(block_key),
    });

    outcomes
}

/// Draw the block selection the tab in front keeps in each of its panes:
/// the selected block in its terminal, no selection in the others.
fn sync_selection(state: &TerminalWorkspaceState) -> Vec<TerminalOutcome> {
    let Some(tab) = state.active_tab() else {
        return Vec::new();
    };
    tab.terminals()
        .values()
        .map(|entry| {
            let command = match tab.selected_block_key_in(entry.terminal().id) {
                Some(block_key) => BlockCommand::Select(block_key),
                None => BlockCommand::ClearSelection,
            };

            TerminalOutcome::RunBlockCommand {
                terminal: entry.terminal().widget_id().clone(),
                command,
            }
        })
        .collect()
}

fn update_block_selection(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    event: &TerminalEvent,
) -> Vec<TerminalOutcome> {
    use otty_ui_term::Event::*;

    match event {
        BlockSelected { block_key, .. } => {
            let terminal_id = *event.terminal_id();
            let other_widget_ids: Vec<_> = state
                .tab(tab_id)
                .map(|tab| {
                    tab.terminals()
                        .values()
                        .filter(|e| e.terminal().id != terminal_id)
                        .map(|e| e.terminal().widget_id().clone())
                        .collect()
                })
                .unwrap_or_default();

            let _ = with_terminal_tab(state, tab_id, |tab| {
                tab.set_selected_block(terminal_id, *block_key);
                StateCommand::None
            });

            other_widget_ids
                .into_iter()
                .map(|id| TerminalOutcome::RunBlockCommand {
                    terminal: id,
                    command: BlockCommand::ClearSelection,
                })
                .collect()
        },
        BlockSelectionCleared { .. } => {
            let terminal_id = *event.terminal_id();
            with_terminal_tab(state, tab_id, |tab| {
                tab.clear_selected_block_for_terminal(terminal_id);
                StateCommand::None
            })
        },
        _ => Vec::new(),
    }
}

/// Change the tab `tab_id` with `f` and read what it asks for, then the new
/// title of the tab if it may have changed. Nothing happens for a tab that
/// is gone.
fn with_terminal_tab<F>(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    f: F,
) -> Vec<TerminalOutcome>
where
    F: FnOnce(&mut TerminalTabState) -> StateCommand,
{
    let Some(tab) = state.tab_mut(tab_id) else {
        return Vec::new();
    };

    let command = f(tab);
    let title_changed =
        tab.take_label_change().then(|| title_changed(tab_id, tab));

    let mut outcomes = Vec::new();
    push_command(command, &mut outcomes);
    outcomes.extend(title_changed);

    outcomes
}

/// Report the tab's current title and folder name to the tabs.
fn title_changed(tab_id: u64, tab: &TerminalTabState) -> TerminalOutcome {
    TerminalOutcome::TitleChanged {
        tab_id,
        title: tab.title().to_string(),
        folder: tab.folder().map(str::to_string),
    }
}

/// Turn what a tab asked for into the outcomes the binary reads.
fn push_command(command: StateCommand, outcomes: &mut Vec<TerminalOutcome>) {
    match command {
        StateCommand::None => {},
        StateCommand::FocusTerminal(id) => {
            outcomes.push(TerminalOutcome::FocusTerminal(id));
        },
        StateCommand::SelectHovered(id) => {
            outcomes.push(TerminalOutcome::RunBlockCommand {
                terminal: id,
                command: BlockCommand::SelectHovered,
            });
        },
        StateCommand::ClearSelection(id) => {
            outcomes.push(TerminalOutcome::RunBlockCommand {
                terminal: id,
                command: BlockCommand::ClearSelection,
            });
        },
        StateCommand::FocusNothing => {
            outcomes.push(TerminalOutcome::TakeKeyboard);
        },
        StateCommand::SplitFailed(error) => {
            outcomes.push(TerminalOutcome::Failed(i18n::with_error(
                Key::TplSplitPaneFailed,
                &error,
            )));
        },
        StateCommand::NoRoomToSplit => {
            outcomes.push(TerminalOutcome::Failed(String::from(i18n::t(
                Key::NoRoomForPane,
            ))));
        },
        StateCommand::CloseTab { tab_id } => {
            outcomes.push(TerminalOutcome::CloseTab { tab_id });
        },
        StateCommand::Batch(commands) => {
            for command in commands {
                push_command(command, outcomes);
            }
        },
    }
}

fn reindex_terminal_tabs(
    state: &TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
) {
    terminal_to_tab.clear();
    for (&tab_id, tab) in state.tabs() {
        for terminal_id in tab.terminals().keys().copied() {
            terminal_to_tab.insert(terminal_id, tab_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::blocks::{command_history, key_of, snapshot_in};
    use otty_app_testing::terminal::ScriptedTerminals;
    use otty_libterm::escape::NamedPrivateMode;
    use otty_libterm::surface::{
        SnapshotOwned, Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
    };
    use otty_ui_kit::components::target_dot::TargetKind;
    use otty_ui_term::settings::{
        LocalSessionOptions, SSHSessionOptions, SessionKind, Settings,
    };
    use otty_ui_term::{BlockCommand, BlockKey};

    use super::reduce;
    use crate::ctx::TerminalCtx;
    use crate::event::{TerminalIntent, TerminalOutcome};
    use crate::state::TerminalWorkspaceState;
    use crate::types::{PaneMachine, StartTerminal, Terminal, TerminalKind};

    #[cfg(unix)]
    const VALID_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const VALID_SHELL_PATH: &str = "cmd.exe";

    /// The terminals of the tests, with what the feature keeps beside the
    /// state.
    struct Workspace {
        state: TerminalWorkspaceState,
        terminal_to_tab: HashMap<u64, u64>,
        next_id: u64,
        start_terminal: StartTerminal,
        /// What the terminals show next, when their programs are mocks
        /// (see [`Workspace::scripted`]).
        terminals: ScriptedTerminals,
    }

    impl Workspace {
        /// No tab is open; the first terminal gets id 100. A terminal
        /// starts its real program.
        fn new() -> Self {
            Self::starting_terminals_with(
                Rc::new(Terminal::new),
                ScriptedTerminals::default(),
            )
        }

        /// As [`Workspace::new`], but the program of every terminal is a
        /// mock: no shell starts, and the test decides what a terminal
        /// shows with [`Workspace::report_folder`].
        fn scripted() -> Self {
            let terminals = ScriptedTerminals::default();

            Self::starting_terminals_with(
                Rc::new(terminals.starter()),
                terminals,
            )
        }

        fn starting_terminals_with(
            start_terminal: StartTerminal,
            terminals: ScriptedTerminals,
        ) -> Self {
            Self {
                state: TerminalWorkspaceState::default(),
                terminal_to_tab: HashMap::new(),
                next_id: 100,
                start_terminal,
                terminals,
            }
        }

        fn reduce(
            &mut self,
            intent: TerminalIntent,
            ctx: &TerminalCtx,
        ) -> Vec<TerminalOutcome> {
            reduce(
                &mut self.state,
                &mut self.terminal_to_tab,
                &mut self.next_id,
                &self.start_terminal,
                intent,
                ctx,
            )
        }

        /// Open tab `tab_id` running `program` as `kind`, in `ctx`.
        fn open(
            &mut self,
            tab_id: u64,
            program: &str,
            kind: TerminalKind,
            ctx: &TerminalCtx,
        ) -> Vec<TerminalOutcome> {
            self.reduce(
                open_tab(tab_id, settings_with_program(program), kind),
                ctx,
            )
        }

        /// Open the shell tab `tab_id`; it becomes the active tab.
        fn open_shell(&mut self, tab_id: u64) -> Vec<TerminalOutcome> {
            self.open(
                tab_id,
                VALID_SHELL_PATH,
                TerminalKind::Shell,
                &default_ctx(),
            )
        }

        /// The number of terminals in tab 1.
        fn terminal_count(&self) -> usize {
            self.state
                .tab(1)
                .expect("tab must exist after opening")
                .terminals()
                .len()
        }

        fn focused_terminal_id(&self, tab_id: u64) -> Option<u64> {
            self.state
                .tab(tab_id)
                .and_then(|tab| tab.focused_terminal_id())
        }

        fn has_context_menu(&self) -> bool {
            self.state
                .tab(1)
                .and_then(|tab| tab.context_menu())
                .is_some()
        }

        /// Make terminal `terminal_id` report `cwd`, the way the shell
        /// integration does after a prompt.
        ///
        /// The workspace must be [`Workspace::scripted`]: only there the
        /// test decides what the program of a terminal has ready.
        fn report_folder(
            &mut self,
            terminal_id: u64,
            cwd: &str,
        ) -> Vec<TerminalOutcome> {
            let snapshot = snapshot_in(cwd);
            assert!(
                self.terminal_to_tab.contains_key(&terminal_id),
                "terminal {terminal_id} should be open"
            );

            self.terminals.report(terminal_id, snapshot);

            self.reduce(
                TerminalIntent::Widget(otty_ui_term::Event::ContentSync {
                    id: terminal_id,
                }),
                &default_ctx(),
            )
        }

        /// Open the pane menu on the pane of terminal `terminal_id` in tab 1.
        fn open_pane_menu(&mut self, terminal_id: u64) {
            let pane = self
                .state
                .tab(1)
                .and_then(|tab| tab.pane_for_terminal(terminal_id))
                .expect("the terminal must have a pane");

            let _ = self.reduce(
                TerminalIntent::OpenContextMenu {
                    tab_id: 1,
                    pane,
                    terminal_id,
                },
                &default_ctx(),
            );

            assert!(self.has_context_menu());
        }
    }

    fn settings_with_program(program: &str) -> Settings {
        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(program),
            ),
        );
        settings
    }

    fn machine() -> PaneMachine {
        PaneMachine {
            name: String::from("build-box"),
            target: TargetKind::Local,
            home: None,
        }
    }

    fn open_tab(
        tab_id: u64,
        settings: Settings,
        kind: TerminalKind,
    ) -> TerminalIntent {
        TerminalIntent::OpenTab {
            tab_id,
            title: String::from("Shell"),
            settings: Box::new(settings),
            kind,
            machine: machine(),
        }
    }

    fn default_ctx() -> TerminalCtx {
        TerminalCtx {
            pane_grid_size: Size::ZERO,
            screen_size: Size::ZERO,
            cursor: Point::ORIGIN,
            equalize_panes: true,
            is_terminal_mode: true,
            is_overlay_open: false,
            is_other_overlay_open: false,
        }
    }

    fn overlay_ctx() -> TerminalCtx {
        TerminalCtx {
            is_overlay_open: true,
            ..default_ctx()
        }
    }

    fn containers_ctx() -> TerminalCtx {
        TerminalCtx {
            is_terminal_mode: false,
            ..default_ctx()
        }
    }

    /// Tab 1 with one shell pane, terminal 100, on a machine whose home
    /// folder is `home`.
    fn shell_tab_with_home(home: Option<&str>) -> Workspace {
        let mut workspace = Workspace::scripted();
        let intent = TerminalIntent::OpenTab {
            tab_id: 1,
            title: String::from("Shell"),
            settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
            kind: TerminalKind::Shell,
            machine: PaneMachine {
                home: home.map(PathBuf::from),
                ..machine()
            },
        };
        let _ = workspace.reduce(intent, &default_ctx());

        workspace
    }

    /// The folder the label of the focused pane of tab 1 shows.
    fn focused_folder_path(workspace: &Workspace) -> Option<String> {
        workspace
            .state
            .tab(1)
            .and_then(|tab| tab.focused_terminal_entry())
            .and_then(|entry| entry.label().folder_path())
            .map(String::from)
    }

    /// Tab 1 with one shell pane, terminal 100.
    fn one_shell_tab() -> Workspace {
        let mut workspace = Workspace::scripted();
        let _ = workspace.open_shell(1);

        workspace
    }

    /// Tab 1 with one shell pane, terminal 100, whose program is a mock:
    /// the tab of the tests that decide what a terminal shows.
    fn one_scripted_tab() -> Workspace {
        let mut workspace = Workspace::scripted();
        let _ = workspace.open_shell(1);

        workspace
    }

    /// Split the only pane of [`one_scripted_tab`]. The new pane shows
    /// terminal 101 next to terminal 100 and has the focus.
    fn split_scripted_tab() -> Workspace {
        let mut workspace = one_scripted_tab();
        let _ = workspace.reduce(split_right(), &default_ctx());
        assert_eq!(workspace.terminal_count(), 2);

        workspace
    }

    /// Split the only pane of [`one_shell_tab`]. The new pane runs terminal
    /// 101 next to terminal 100 and has the focus.
    fn split_shell_tab() -> Workspace {
        let mut workspace = one_shell_tab();
        let _ = workspace.reduce(split_right(), &default_ctx());
        assert_eq!(workspace.terminal_count(), 2);

        workspace
    }

    /// Tab 1 with one shell pane in a pane grid of `grid`, as the binary
    /// measures it for the window.
    fn shell_tab_in_grid(grid: Size) -> Workspace {
        let mut workspace = Workspace::scripted();
        let ctx = TerminalCtx {
            pane_grid_size: grid,
            ..default_ctx()
        };
        let _ = workspace.open(1, VALID_SHELL_PATH, TerminalKind::Shell, &ctx);

        workspace
    }

    fn split_right() -> TerminalIntent {
        TerminalIntent::SplitFocusedPane {
            axis: pane_grid::Axis::Vertical,
        }
    }

    /// The shell of the terminal `id` exits, which closes its pane.
    fn shell_exits(id: u64) -> TerminalIntent {
        TerminalIntent::Widget(otty_ui_term::Event::Shutdown {
            id,
            exit_status: std::process::ExitStatus::default(),
        })
    }

    /// The text of the one failure among `outcomes`.
    fn failure_text(outcomes: &[TerminalOutcome]) -> Option<&str> {
        let mut failures =
            outcomes.iter().filter_map(|outcome| match outcome {
                TerminalOutcome::Failed(text) => Some(text.as_str()),
                _ => None,
            });
        let text = failures.next();

        assert!(failures.next().is_none(), "one failure at most");
        text
    }

    fn closes_tab(outcomes: &[TerminalOutcome], id: u64) -> bool {
        outcomes.iter().any(|outcome| {
            matches!(outcome, TerminalOutcome::CloseTab { tab_id } if *tab_id == id)
        })
    }

    fn reports_a_started_tab(outcomes: &[TerminalOutcome]) -> bool {
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, TerminalOutcome::TabStarted))
    }

    fn focuses_a_terminal(outcomes: &[TerminalOutcome]) -> bool {
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, TerminalOutcome::FocusTerminal(_)))
    }

    fn followed_folders(outcomes: &[TerminalOutcome]) -> Vec<PathBuf> {
        outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                TerminalOutcome::FolderFollowed { cwd } => Some(cwd.clone()),
                _ => None,
            })
            .collect()
    }

    /// Return the axis of the split whose two halves are exactly these
    /// panes, which is what "b was split off a" looks like in the layout
    /// tree. `None` when they are not each other's split sibling.
    fn sibling_split_axis(
        layout: &pane_grid::Node,
        left: pane_grid::Pane,
        right: pane_grid::Pane,
    ) -> Option<pane_grid::Axis> {
        let pane_grid::Node::Split { axis, a, b, .. } = layout else {
            return None;
        };

        let siblings = matches!(
            (a.as_ref(), b.as_ref()),
            (pane_grid::Node::Pane(x), pane_grid::Node::Pane(y))
                if (*x == left && *y == right) || (*x == right && *y == left)
        );

        if siblings {
            return Some(*axis);
        }

        sibling_split_axis(a, left, right)
            .or_else(|| sibling_split_axis(b, left, right))
    }

    /// The one test that starts a real shell, so that starting a terminal
    /// is not broken without any test noticing. The others use mocks: a
    /// real shell needs a pty and a process, which a busy machine may
    /// refuse.
    #[test]
    fn given_open_tab_command_when_reduced_then_state_stores_tab() {
        let mut workspace = Workspace::new();

        let _ = workspace.open_shell(1);

        assert!(workspace.state.tab(1).is_some());
        assert_eq!(workspace.state.active_tab_id(), Some(1));
        assert_eq!(workspace.terminal_to_tab.get(&100), Some(&1));
    }

    /// A tab reports that it started only once its terminal runs, so the app
    /// shows Terminal mode for a tab that is really there. The tab is named
    /// after its pane, and the new pane takes the keyboard.
    #[test]
    fn a_tab_whose_terminal_starts_reports_it() {
        let mut workspace = Workspace::scripted();

        let outcomes = workspace.open_shell(1);

        assert!(matches!(
            outcomes.as_slice(),
            [
                TerminalOutcome::TitleChanged { tab_id: 1, title, folder: None },
                TerminalOutcome::TabStarted,
                TerminalOutcome::FocusTerminal(_),
            ] if title == "Shell"
        ));
    }

    /// Open tab 99 whose terminal cannot start, and return what the reducer
    /// reported. The workspace must be [`Workspace::scripted`].
    fn open_broken_tab(
        workspace: &mut Workspace,
        kind: TerminalKind,
    ) -> Vec<TerminalOutcome> {
        workspace.terminals.fail_next_start();

        workspace.reduce(
            TerminalIntent::OpenTab {
                tab_id: 99,
                title: String::from("Broken command"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind,
                machine: machine(),
            },
            &default_ctx(),
        )
    }

    /// A launch whose terminal cannot start closes its tab and tells the user
    /// which launch failed. It reports no started tab, so the mode stays.
    #[test]
    fn a_command_tab_that_cannot_start_closes_and_tells_the_user() {
        let mut workspace = Workspace::scripted();

        let outcomes = open_broken_tab(
            &mut workspace,
            TerminalKind::Command {
                exit_behavior: ExitBehavior::Close,
            },
        );

        assert!(workspace.state.tab(99).is_none());
        assert_eq!(workspace.terminal_to_tab.len(), 0);
        assert!(closes_tab(&outcomes, 99));
        let text = failure_text(&outcomes).expect("the user should be told");
        assert!(
            text.starts_with(&i18n::launch_failed("Broken command", "")),
            "{text}"
        );
        assert!(!reports_a_started_tab(&outcomes));
    }

    /// A shell tab whose terminal cannot start closes and tells the user.
    #[test]
    fn a_shell_tab_that_cannot_start_closes_and_tells_the_user() {
        let mut workspace = Workspace::scripted();

        let outcomes = open_broken_tab(&mut workspace, TerminalKind::Shell);

        assert!(workspace.state.tab(99).is_none());
        assert!(closes_tab(&outcomes, 99));
        let text = failure_text(&outcomes).expect("the user should be told");
        assert!(
            text.starts_with(&i18n::with_error(Key::TplTerminalInitFailed, "")),
            "{text}"
        );
        assert!(!reports_a_started_tab(&outcomes));
    }

    /// A split whose new terminal cannot start keeps the one pane and tells
    /// the user.
    #[test]
    fn a_split_whose_terminal_cannot_start_keeps_one_pane_and_tells_the_user() {
        let mut workspace = Workspace::scripted();
        let _ = workspace.open_shell(1);
        workspace.terminals.fail_next_start();

        let outcomes = workspace.reduce(split_right(), &default_ctx());

        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(
            workspace.state.tab(1).map(|tab| tab.panes().len()),
            Some(1)
        );
        assert_eq!(workspace.terminal_to_tab.get(&101), None);
        let text = failure_text(&outcomes).expect("the user should be told");
        assert!(
            text.starts_with(&i18n::with_error(Key::TplSplitPaneFailed, "")),
            "{text}"
        );
    }

    #[test]
    fn given_sync_pane_grid_size_when_reduced_then_all_tab_grid_sizes_update() {
        let mut workspace = shell_tab_in_grid(Size::new(120.0, 80.0));
        let sync_ctx = TerminalCtx {
            pane_grid_size: Size::new(480.0, 320.0),
            ..default_ctx()
        };

        let outcomes =
            workspace.reduce(TerminalIntent::SyncPaneGridSize, &sync_ctx);

        let tab = workspace
            .state
            .tab(1)
            .expect("tab must exist after opening");
        assert_eq!(tab.grid_size(), Size::new(480.0, 320.0));
        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_missing_tab_when_pane_clicked_then_reducer_ignores_event() {
        let mut workspace = Workspace::scripted();
        let (_grid, pane) = pane_grid::State::new(1_u64);

        let outcomes = workspace.reduce(
            TerminalIntent::PaneClicked { tab_id: 999, pane },
            &default_ctx(),
        );

        assert!(workspace.state.tab(999).is_none());
        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_close_all_context_menus_when_no_menus_open_then_nothing_happens() {
        let mut workspace = one_shell_tab();

        let outcomes = workspace
            .reduce(TerminalIntent::CloseAllContextMenus, &default_ctx());

        assert!(outcomes.is_empty());
    }

    /// The pane menu takes the keyboard from its terminal, so typed keys do
    /// not reach the shell and Esc reaches the frame. It opens at the
    /// pointer, inside the main area, and selects the hovered block.
    #[test]
    fn opening_the_pane_menu_takes_the_keyboard_focus() {
        let mut workspace = one_shell_tab();
        let pane = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");
        let ctx = TerminalCtx {
            cursor: Point::new(40.0, 60.0),
            screen_size: Size::new(1024.0, 700.0),
            ..default_ctx()
        };

        let outcomes = workspace.reduce(
            TerminalIntent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 100,
            },
            &ctx,
        );

        let menu = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.context_menu())
            .expect("the menu should be open");
        assert_eq!(menu.cursor(), Point::new(40.0, 60.0));
        assert_eq!(menu.grid_size(), Size::new(1024.0, 700.0));
        assert!(matches!(
            outcomes.as_slice(),
            [
                TerminalOutcome::RunBlockCommand {
                    command: BlockCommand::SelectHovered,
                    ..
                },
                TerminalOutcome::TakeKeyboard,
                TerminalOutcome::TitleChanged { tab_id: 1, .. },
            ]
        ));
    }

    /// The menu closes on a press outside it: the selection of every pane
    /// clears and the focused pane takes the keyboard back.
    #[test]
    fn closing_the_pane_menu_gives_the_pane_its_keys_back() {
        let mut workspace = one_shell_tab();
        workspace.open_pane_menu(100);

        let outcomes = workspace.reduce(
            TerminalIntent::CloseContextMenu { tab_id: 1 },
            &default_ctx(),
        );

        assert!(!workspace.has_context_menu());
        assert!(matches!(
            outcomes.as_slice(),
            [
                TerminalOutcome::RunBlockCommand {
                    command: BlockCommand::ClearSelection,
                    ..
                },
                TerminalOutcome::FocusTerminal(_),
                TerminalOutcome::TitleChanged { tab_id: 1, .. },
            ]
        ));
    }

    #[test]
    fn pressing_the_fold_arrow_of_a_block_asks_its_terminal_to_fold_it() {
        let mut workspace = one_shell_tab();
        let widget_id = widget_id_of(&workspace, 100);

        let outcomes = workspace.reduce(
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 100,
                command: BlockCommand::ToggleFold(key_of("cmd-3")),
            },
            &default_ctx(),
        );

        assert!(
            matches!(
                outcomes.as_slice(),
                [TerminalOutcome::RunBlockCommand {
                    terminal,
                    command: BlockCommand::ToggleFold(block_key),
                }] if *terminal == widget_id && *block_key == key_of("cmd-3")
            ),
            "got {outcomes:?}"
        );
    }

    #[test]
    fn pressing_go_to_start_in_the_sticky_header_asks_its_terminal_to_bring_the_block_into_view()
     {
        let mut workspace = one_shell_tab();
        let widget_id = widget_id_of(&workspace, 100);

        let outcomes = workspace.reduce(
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 100,
                command: BlockCommand::ScrollTo(key_of("cmd-3")),
            },
            &default_ctx(),
        );

        assert!(
            matches!(
                outcomes.as_slice(),
                [TerminalOutcome::RunBlockCommand {
                    terminal,
                    command: BlockCommand::ScrollTo(block_key),
                }] if *terminal == widget_id && *block_key == key_of("cmd-3")
            ),
            "got {outcomes:?}"
        );
    }

    #[test]
    fn pressing_the_fold_arrow_of_a_terminal_that_is_gone_does_nothing() {
        let mut workspace = one_shell_tab();

        let outcomes = workspace.reduce(
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 999,
                command: BlockCommand::ToggleFold(key_of("cmd-3")),
            },
            &default_ctx(),
        );

        assert!(outcomes.is_empty(), "got {outcomes:?}");
    }

    /// Select the block named `name` of terminal 100 the way a right-click
    /// does, then open the menu of its pane.
    fn open_menu_on_block(workspace: &mut Workspace, name: &str) {
        let _ = workspace.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::BlockSelected {
                id: 100,
                block_key: key_of(name),
            }),
            &default_ctx(),
        );
        workspace.open_pane_menu(100);
    }

    /// `Collapse` and `Expand` in the menu ask the terminal of the block
    /// to fold or unfold it, then the menu closes as on a press outside
    /// it: the selection clears and the pane takes the keyboard back.
    #[test]
    fn the_fold_item_of_the_menu_asks_the_terminal_to_fold_or_unfold_the_block_and_closes_the_menu()
     {
        let mut workspace = one_shell_tab();
        let widget_id = widget_id_of(&workspace, 100);
        open_menu_on_block(&mut workspace, "cmd-3");

        let outcomes = workspace.reduce(
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 100,
                command: BlockCommand::ToggleFold(key_of("cmd-3")),
            },
            &default_ctx(),
        );

        assert!(!workspace.has_context_menu());
        assert!(
            matches!(
                outcomes.as_slice(),
                [
                    TerminalOutcome::RunBlockCommand {
                        terminal,
                        command: BlockCommand::ToggleFold(block_key),
                    },
                    TerminalOutcome::RunBlockCommand {
                        command: BlockCommand::ClearSelection,
                        ..
                    },
                    TerminalOutcome::FocusTerminal(_),
                    TerminalOutcome::TitleChanged { tab_id: 1, .. },
                ] if *terminal == widget_id && *block_key == key_of("cmd-3")
            ),
            "got {outcomes:?}"
        );
    }

    /// `Run again` asks the terminal of the block to type its command at
    /// the prompt and press Enter, then the block selection clears; the
    /// write scrolls the pane to the bottom.
    #[test]
    fn run_again_in_the_menu_sends_the_command_of_the_block_to_its_terminal_and_clears_the_selection()
     {
        let mut workspace = one_shell_tab();
        let widget_id = widget_id_of(&workspace, 100);
        open_menu_on_block(&mut workspace, "cmd-3");

        let outcomes = workspace.reduce(
            TerminalIntent::RunBlockCommand {
                tab_id: 1,
                terminal_id: 100,
                command: BlockCommand::RunAgain(key_of("cmd-3")),
            },
            &default_ctx(),
        );

        assert!(!workspace.has_context_menu());
        assert!(
            workspace
                .state
                .tab(1)
                .is_some_and(|tab| tab.selected_block().is_none())
        );
        assert!(
            matches!(
                outcomes.as_slice(),
                [
                    TerminalOutcome::RunBlockCommand {
                        terminal: run_on,
                        command: BlockCommand::RunAgain(block_key),
                    },
                    TerminalOutcome::RunBlockCommand {
                        terminal: cleared_on,
                        command: BlockCommand::ClearSelection,
                    },
                    ..
                ] if *run_on == widget_id
                    && *cleared_on == widget_id
                    && *block_key == key_of("cmd-3")
            ),
            "got {outcomes:?}"
        );
    }

    #[test]
    fn given_focus_active_with_no_tabs_when_reduced_then_nothing_happens() {
        let mut workspace = Workspace::scripted();
        let _ = workspace.reduce(
            TerminalIntent::TabActivated { tab_id: 999 },
            &default_ctx(),
        );

        let outcomes =
            workspace.reduce(TerminalIntent::FocusActive, &default_ctx());

        assert!(outcomes.is_empty());
    }

    /// Scenario 13: a menu, a dialog or the inline edit keeps the keyboard
    /// while a palette dismiss or a mode switch asks for the active pane.
    #[test]
    fn focus_active_takes_no_focus_from_an_open_overlay() {
        let mut workspace = one_shell_tab();

        let outcomes =
            workspace.reduce(TerminalIntent::FocusActive, &overlay_ctx());

        assert!(outcomes.is_empty());
    }

    #[test]
    fn focus_active_focuses_the_active_pane_without_an_overlay() {
        let mut workspace = one_shell_tab();

        let outcomes =
            workspace.reduce(TerminalIntent::FocusActive, &default_ctx());

        assert!(matches!(
            outcomes.as_slice(),
            [TerminalOutcome::FocusTerminal(_)]
        ));
    }

    /// After an overlay closes in Containers mode no pane is on screen, so
    /// nothing takes the keyboard.
    #[test]
    fn focus_active_outside_terminal_mode_focuses_nothing() {
        let mut workspace = one_shell_tab();

        let outcomes =
            workspace.reduce(TerminalIntent::FocusActive, &containers_ctx());

        assert!(outcomes.is_empty());
    }

    /// Back in Terminal mode the panes take the grid size the window has
    /// now, and the active pane the keyboard. Leaving it changes nothing.
    #[test]
    fn back_in_terminal_mode_the_panes_resize_and_the_pane_takes_the_keys() {
        let mut workspace = shell_tab_in_grid(Size::new(300.0, 200.0));
        let grown = TerminalCtx {
            pane_grid_size: Size::new(900.0, 600.0),
            ..containers_ctx()
        };

        let left = workspace.reduce(TerminalIntent::ModeChanged, &grown);
        let back = workspace.reduce(
            TerminalIntent::ModeChanged,
            &TerminalCtx {
                is_terminal_mode: true,
                ..grown
            },
        );

        assert!(left.is_empty());
        assert!(matches!(
            back.as_slice(),
            [TerminalOutcome::FocusTerminal(_)]
        ));
        assert_eq!(
            workspace.state.tab(1).map(|tab| tab.grid_size()),
            Some(Size::new(900.0, 600.0))
        );
    }

    /// Scenario 13: a tab that opens in the background, such as a launch
    /// that was started earlier, leaves the keyboard with the overlay.
    #[test]
    fn a_tab_opened_under_an_overlay_leaves_the_focus_alone() {
        let mut workspace = Workspace::scripted();

        let outcomes = workspace.open(
            1,
            VALID_SHELL_PATH,
            TerminalKind::Shell,
            &overlay_ctx(),
        );

        assert!(workspace.state.tab(1).is_some());
        assert!(!focuses_a_terminal(&outcomes));
    }

    /// Scenario 13: the palette, a menu or the SSH prompt is open when the
    /// shell of one pane exits. The pane closes, and the keyboard stays with
    /// the overlay. The pane left is the one the overlay's close focuses.
    #[test]
    fn a_pane_that_exits_under_an_overlay_takes_no_focus() {
        let mut workspace = split_shell_tab();

        let outcomes = workspace.reduce(shell_exits(101), &overlay_ctx());

        assert!(!focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(workspace.focused_terminal_id(1), Some(100));
    }

    #[test]
    fn a_pane_that_exits_in_the_active_tab_focuses_the_pane_left() {
        let mut workspace = split_shell_tab();

        let outcomes = workspace.reduce(shell_exits(101), &default_ctx());

        assert!(focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
    }

    /// A pane exits in a tab the user does not see. The keyboard stays in
    /// the tab the user types in.
    #[test]
    fn a_pane_that_exits_in_a_background_tab_takes_no_focus() {
        let mut workspace = split_shell_tab();
        let _ = workspace.open_shell(2);

        let outcomes = workspace.reduce(shell_exits(101), &default_ctx());

        assert!(!focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
    }

    /// Close pane in the pane menu closes the menu with the pane, so the
    /// pane left takes the keyboard although the menu was open.
    #[test]
    fn close_pane_in_the_pane_menu_focuses_the_pane_left() {
        let mut workspace = split_shell_tab();
        let pane = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.pane_for_terminal(101))
            .expect("the split must have created a pane");
        workspace.open_pane_menu(101);

        let outcomes = workspace.reduce(
            TerminalIntent::ClosePane { tab_id: 1, pane },
            &overlay_ctx(),
        );

        assert!(focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
        assert!(!workspace.has_context_menu());
    }

    /// The shell of a pane exits while the menu of that pane is the only
    /// overlay. The menu closes with the pane, so no overlay is left and the
    /// pane left takes the keyboard, as when the user closes the menu.
    #[test]
    fn a_pane_that_exits_under_its_own_menu_focuses_the_pane_left() {
        let mut workspace = split_shell_tab();
        workspace.open_pane_menu(101);

        let outcomes = workspace.reduce(shell_exits(101), &overlay_ctx());

        assert!(focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
        assert!(!workspace.has_context_menu());
        assert_eq!(workspace.focused_terminal_id(1), Some(100));
    }

    /// The SSH prompt lies over the menu of a pane when its shell exits.
    /// The menu closes with the pane, and the keyboard stays with the
    /// prompt.
    #[test]
    fn a_pane_that_exits_under_its_menu_and_the_ssh_prompt_takes_no_focus() {
        let mut workspace = split_shell_tab();
        workspace.open_pane_menu(101);
        let ctx = TerminalCtx {
            is_other_overlay_open: true,
            ..overlay_ctx()
        };

        let outcomes = workspace.reduce(shell_exits(101), &ctx);

        assert!(!focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
        assert!(!workspace.has_context_menu());
        assert_eq!(workspace.focused_terminal_id(1), Some(100));
    }

    /// The shell of one pane exits while the menu of another pane is open.
    /// That menu stays open and keeps the keyboard it took when it opened.
    #[test]
    fn a_pane_that_exits_leaves_the_menu_of_another_pane_open() {
        let mut workspace = split_shell_tab();
        workspace.open_pane_menu(100);

        let outcomes = workspace.reduce(shell_exits(101), &overlay_ctx());

        assert!(!focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(
            workspace
                .state
                .tab(1)
                .and_then(|tab| tab.context_menu())
                .map(|menu| menu.terminal_id()),
            Some(100)
        );
    }

    /// The last pane of a tab closes with its shell: the tab closes.
    #[test]
    fn the_last_pane_that_exits_closes_its_tab() {
        let mut workspace = one_shell_tab();

        let outcomes = workspace.reduce(shell_exits(100), &default_ctx());

        assert!(closes_tab(&outcomes, 1));
    }

    #[test]
    fn given_exited_split_pane_when_key_pressed_then_its_terminal_is_unmapped()
    {
        let mut workspace = Workspace::scripted();
        let _ = workspace.open(
            1,
            VALID_SHELL_PATH,
            TerminalKind::Command {
                exit_behavior: ExitBehavior::KeepOpen,
            },
            &default_ctx(),
        );
        let _ = workspace.reduce(split_right(), &default_ctx());

        for event in [
            otty_ui_term::Event::Shutdown {
                id: 101,
                exit_status: std::process::ExitStatus::default(),
            },
            otty_ui_term::Event::Write {
                id: 101,
                data: b"q".to_vec(),
            },
        ] {
            let _ =
                workspace.reduce(TerminalIntent::Widget(event), &default_ctx());
        }

        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(workspace.terminal_to_tab.get(&101), None);
        assert_eq!(workspace.terminal_to_tab.get(&100), Some(&1));
    }

    #[test]
    fn given_split_focused_pane_when_reduced_then_the_focused_pane_of_the_active_tab_is_split()
     {
        let mut workspace = one_shell_tab();
        let focused = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");

        let outcomes = workspace.reduce(split_right(), &default_ctx());

        let tab = workspace
            .state
            .tab(1)
            .expect("tab must exist after opening");
        let created = tab
            .pane_for_terminal(101)
            .expect("the split must have created a terminal");
        assert_eq!(
            sibling_split_axis(tab.panes().layout(), focused, created),
            Some(pane_grid::Axis::Vertical)
        );
        assert_eq!(workspace.terminal_to_tab.get(&101), Some(&1));
        assert!(focuses_a_terminal(&outcomes));
    }

    /// The launch editor is the active tab, or the active tab is gone: no
    /// terminal tab is in front, so nothing is split.
    #[test]
    fn given_split_focused_pane_without_an_active_terminal_tab_when_reduced_then_nothing_is_split()
     {
        let mut workspace = one_shell_tab();
        let _ = workspace
            .reduce(TerminalIntent::TabActivated { tab_id: 7 }, &default_ctx());

        let outcomes = workspace.reduce(split_right(), &default_ctx());

        assert_eq!(workspace.terminal_count(), 1);
        assert!(outcomes.is_empty());
    }

    /// Tab 1 on `session`, whose terminals start through a starter that
    /// keeps the program of each one it starts.
    fn recording_tab(
        session: SessionKind,
    ) -> (Workspace, Rc<RefCell<Vec<String>>>) {
        let terminals = ScriptedTerminals::default();
        let starter = terminals.starter();
        let started = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&started);
        let start: StartTerminal = Rc::new(move |id, settings: Settings| {
            let program = match &settings.engine.session {
                SessionKind::Local(options) => options.program().clone(),
                SessionKind::Ssh(_) => String::from("ssh"),
            };
            record.borrow_mut().push(program);
            starter(id, settings)
        });
        let mut workspace =
            Workspace::starting_terminals_with(start, terminals);

        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(session);
        let _ = workspace
            .reduce(open_tab(1, settings, TerminalKind::Shell), &default_ctx());

        (workspace, started)
    }

    fn split_right_running(program: &str) -> TerminalIntent {
        TerminalIntent::SplitFocusedPaneWith {
            axis: pane_grid::Axis::Vertical,
            settings: Box::new(settings_with_program(program)),
        }
    }

    /// An agent continued in a split runs in the new pane; the tab's own
    /// shell stays what the next plain split starts.
    #[test]
    fn a_split_with_its_own_settings_runs_them_in_the_new_pane() {
        let (mut workspace, started) =
            recording_tab(SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(VALID_SHELL_PATH),
            ));

        let outcomes =
            workspace.reduce(split_right_running("agent"), &default_ctx());
        let _ = workspace.reduce(split_right(), &default_ctx());

        assert_eq!(workspace.terminal_count(), 3);
        assert_eq!(
            *started.borrow(),
            [VALID_SHELL_PATH, "agent", VALID_SHELL_PATH]
        );
        assert!(focuses_a_terminal(&outcomes));
        assert_eq!(workspace.terminal_to_tab.get(&101), Some(&1));
    }

    /// A pane of another machine must not run a program of this one.
    #[test]
    fn a_split_with_its_own_settings_leaves_a_tab_of_another_machine_alone() {
        let (mut workspace, started) =
            recording_tab(SessionKind::from_ssh_options(
                SSHSessionOptions::default().with_host("build-box"),
            ));

        let outcomes =
            workspace.reduce(split_right_running("agent"), &default_ctx());

        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(*started.borrow(), ["ssh"]);
        assert!(outcomes.is_empty());
    }

    /// Owner: a split with no room is refused with a toast, before its
    /// terminal starts.
    #[test]
    fn a_split_with_no_room_starts_no_terminal_and_tells_the_user() {
        let mut workspace = shell_tab_in_grid(Size::new(150.0, 600.0));

        let outcomes = workspace.reduce(split_right(), &default_ctx());

        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(
            workspace.state.tab(1).map(|tab| tab.panes().len()),
            Some(1)
        );
        assert_eq!(workspace.terminal_to_tab.len(), 1);
        assert_eq!(failure_text(&outcomes), Some(i18n::t(Key::NoRoomForPane)));
    }

    /// The pane menu and the New session menu split a given pane; they go
    /// through the same check.
    #[test]
    fn a_pane_menu_split_with_no_room_is_refused_the_same_way() {
        let mut workspace = shell_tab_in_grid(Size::new(800.0, 150.0));
        let pane = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");

        let outcomes = workspace.reduce(
            TerminalIntent::SplitPane {
                tab_id: 1,
                pane,
                axis: pane_grid::Axis::Horizontal,
            },
            &default_ctx(),
        );

        assert_eq!(workspace.terminal_count(), 1);
        assert_eq!(failure_text(&outcomes), Some(i18n::t(Key::NoRoomForPane)));
    }

    /// Two panes of 100 px and the line between them fit 203 px exactly,
    /// with the two side panel lines; a third pane does not.
    #[test]
    fn a_split_that_fits_still_happens() {
        let mut workspace = shell_tab_in_grid(Size::new(203.0, 300.0));

        let _ = workspace.reduce(split_right(), &default_ctx());
        assert_eq!(workspace.terminal_count(), 2);
        let second = workspace.reduce(split_right(), &default_ctx());

        assert_eq!(workspace.terminal_count(), 2);
        assert_eq!(failure_text(&second), Some(i18n::t(Key::NoRoomForPane)));
    }

    /// The folder of the active local terminal reaches the Files section
    /// when its shell reports it, a pane is clicked, a pane closes or a
    /// refused split leaves it in front, each time once.
    #[test]
    fn intents_that_may_change_the_active_folder_follow_it() {
        let mut workspace = split_scripted_tab();
        let first = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.pane_for_terminal(100))
            .expect("the first pane");
        let second = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.pane_for_terminal(101))
            .expect("the second pane");
        let reported = {
            let _ = workspace.reduce(
                TerminalIntent::PaneClicked {
                    tab_id: 1,
                    pane: first,
                },
                &default_ctx(),
            );
            workspace.report_folder(100, "/tmp/otty-first")
        };
        let clicked = workspace.reduce(
            TerminalIntent::PaneClicked {
                tab_id: 1,
                pane: first,
            },
            &default_ctx(),
        );
        let closed = workspace.reduce(
            TerminalIntent::ClosePane {
                tab_id: 1,
                pane: second,
            },
            &default_ctx(),
        );
        let narrow = TerminalCtx {
            pane_grid_size: Size::new(150.0, 600.0),
            ..default_ctx()
        };
        let _ = workspace.reduce(TerminalIntent::SyncPaneGridSize, &narrow);
        let no_room = workspace.reduce(split_right(), &narrow);

        let first_folder = vec![PathBuf::from("/tmp/otty-first")];
        assert_eq!(followed_folders(&reported), first_folder);
        assert_eq!(followed_folders(&clicked), first_folder);
        assert_eq!(followed_folders(&closed), first_folder);
        assert!(matches!(
            no_room.as_slice(),
            [
                TerminalOutcome::Failed(_),
                TerminalOutcome::FolderFollowed { .. },
            ]
        ));
    }

    /// The folder of a pane that is not in front, or of a pane that has
    /// reported none, is not followed.
    #[test]
    fn a_folder_not_in_front_is_not_followed() {
        let mut workspace = split_scripted_tab();

        let background = workspace.report_folder(100, "/tmp/otty-first");
        let without_folder = workspace.reduce(
            TerminalIntent::PaneClicked {
                tab_id: 1,
                pane: workspace
                    .state
                    .tab(1)
                    .and_then(|tab| tab.pane_for_terminal(101))
                    .expect("the second pane"),
            },
            &default_ctx(),
        );

        assert!(followed_folders(&background).is_empty());
        assert!(followed_folders(&without_folder).is_empty());
    }

    /// A pane grid resize does not change the active folder, so the
    /// explorer is left alone.
    #[test]
    fn a_pane_grid_resize_does_not_follow_the_folder() {
        let mut workspace = one_scripted_tab();
        let _ = workspace.report_folder(100, "/tmp/otty-first");

        let outcomes =
            workspace.reduce(TerminalIntent::SyncPaneGridSize, &default_ctx());

        assert!(followed_folders(&outcomes).is_empty());
    }

    /// A background tab comes to the front: its pane takes the keyboard,
    /// the block selection of its panes is drawn again and the Files
    /// section follows its folder.
    #[test]
    fn an_activated_tab_focuses_its_pane_and_follows_its_folder() {
        let mut workspace = one_scripted_tab();
        let _ = workspace.report_folder(100, "/tmp/otty-first");
        let _ = workspace.open_shell(2);

        let outcomes = workspace
            .reduce(TerminalIntent::TabActivated { tab_id: 1 }, &default_ctx());

        assert_eq!(workspace.state.active_tab_id(), Some(1));
        assert!(matches!(
            outcomes.as_slice(),
            [
                TerminalOutcome::FocusTerminal(_),
                TerminalOutcome::RunBlockCommand {
                    command: BlockCommand::ClearSelection,
                    ..
                },
                TerminalOutcome::FolderFollowed { cwd },
            ] if cwd == &PathBuf::from("/tmp/otty-first")
        ));
    }

    /// The launch editor comes to the front: it is no terminal tab, so no
    /// pane takes the keyboard and the folder stays.
    #[test]
    fn an_activated_launch_editor_moves_no_focus() {
        let mut workspace = one_scripted_tab();
        let _ = workspace.report_folder(100, "/tmp/otty-first");

        let outcomes = workspace
            .reduce(TerminalIntent::TabActivated { tab_id: 7 }, &default_ctx());

        assert_eq!(workspace.state.active_tab_id(), Some(7));
        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_tab_closed_command_when_reduced_then_tab_is_removed() {
        let mut workspace = one_shell_tab();

        let outcomes = workspace.reduce(
            TerminalIntent::TabClosed {
                tab_id: 1,
                new_active_id: None,
            },
            &default_ctx(),
        );

        assert!(workspace.state.tab(1).is_none());
        assert_eq!(workspace.terminal_to_tab.get(&100), None);
        assert_eq!(workspace.state.active_tab_id(), None);
        assert!(outcomes.is_empty());
    }

    /// The active tab closes: the terminal decides that the tab in front now
    /// takes the keyboard, its selection is drawn again and the Files section
    /// follows its folder.
    #[test]
    fn a_closed_tab_gives_the_tab_in_front_the_keys_and_its_folder() {
        let mut workspace = one_scripted_tab();
        let _ = workspace.report_folder(100, "/tmp/otty-first");
        let _ = workspace.open_shell(2);

        let outcomes = workspace.reduce(
            TerminalIntent::TabClosed {
                tab_id: 2,
                new_active_id: Some(1),
            },
            &default_ctx(),
        );

        assert!(workspace.state.tab(2).is_none());
        assert_eq!(workspace.state.active_tab_id(), Some(1));
        assert!(focuses_a_terminal(&outcomes));
        assert_eq!(
            followed_folders(&outcomes),
            vec![PathBuf::from("/tmp/otty-first")]
        );
    }

    /// The settings of a program of this computer started in `folder`, as
    /// an agent tab or a local launch is.
    fn settings_in_folder(program: &str, folder: &str) -> Settings {
        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default()
                    .with_program(program)
                    .with_working_directory(PathBuf::from(folder)),
            ),
        );

        settings
    }

    /// An agent tab runs without the shell integration, so it never
    /// reports a folder: the folder it was started in is its folder. Files
    /// follows it each time the tab comes to the front, and the session row
    /// shows its name.
    #[test]
    fn a_command_tab_started_in_a_folder_gives_it_to_files_in_front() {
        let mut workspace = Workspace::scripted();
        let agent = TerminalKind::Command {
            exit_behavior: ExitBehavior::CloseOnSuccess,
        };

        let opened = workspace.reduce(
            open_tab(1, settings_in_folder("agent", "/work/otty"), agent),
            &default_ctx(),
        );
        let _ = workspace.open_shell(2);
        let back = workspace
            .reduce(TerminalIntent::TabActivated { tab_id: 1 }, &default_ctx());

        let project = vec![PathBuf::from("/work/otty")];
        assert_eq!(followed_folders(&opened), project);
        assert_eq!(followed_folders(&back), project);
        assert!(opened.iter().any(|outcome| matches!(
            outcome,
            TerminalOutcome::TitleChanged { tab_id: 1, folder: Some(name), .. }
                if name == "otty"
        )));
        assert_eq!(
            focused_folder_path(&workspace).as_deref(),
            Some("/work/otty")
        );
    }

    /// A shell started in one folder that reports another, after a `cd`,
    /// is in the reported one.
    #[test]
    fn a_reported_folder_wins_over_the_starting_one() {
        let mut workspace = Workspace::scripted();
        let _ = workspace.reduce(
            open_tab(
                1,
                settings_in_folder(VALID_SHELL_PATH, "/work/otty"),
                TerminalKind::Shell,
            ),
            &default_ctx(),
        );

        let reported = workspace.report_folder(100, "/work/otty/src");

        assert_eq!(
            followed_folders(&reported),
            vec![PathBuf::from("/work/otty/src")]
        );
        assert_eq!(
            focused_folder_path(&workspace).as_deref(),
            Some("/work/otty/src")
        );
    }

    /// An SSH tab runs on another machine: its folders are not on this
    /// computer, so Files stays on the last local folder.
    #[test]
    fn an_ssh_tab_does_not_move_files() {
        let mut workspace = Workspace::scripted();
        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(
            SessionKind::from_ssh_options(
                SSHSessionOptions::default().with_host("build-box"),
            ),
        );

        let opened = workspace
            .reduce(open_tab(1, settings, TerminalKind::Shell), &default_ctx());
        let reported = workspace.report_folder(100, "/home/ann/projects");

        assert!(followed_folders(&opened).is_empty());
        assert!(followed_folders(&reported).is_empty());
    }

    /// An agent continued in a split of a shell tab is in its project
    /// folder: Files follows it while its pane has the focus, and the
    /// shell's folder when the shell pane has it.
    #[test]
    fn a_split_started_in_a_folder_gives_its_folder_when_focused() {
        let mut workspace = one_scripted_tab();
        let _ = workspace.report_folder(100, "/tmp/otty-first");
        let shell_pane = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.pane_for_terminal(100))
            .expect("the shell pane");

        let split = workspace.reduce(
            TerminalIntent::SplitFocusedPaneWith {
                axis: pane_grid::Axis::Vertical,
                settings: Box::new(settings_in_folder("agent", "/work/otty")),
            },
            &default_ctx(),
        );
        let agent_pane = workspace
            .state
            .tab(1)
            .and_then(|tab| tab.pane_for_terminal(101))
            .expect("the agent pane");
        let shell = workspace.reduce(
            TerminalIntent::PaneClicked {
                tab_id: 1,
                pane: shell_pane,
            },
            &default_ctx(),
        );
        let agent = workspace.reduce(
            TerminalIntent::PaneClicked {
                tab_id: 1,
                pane: agent_pane,
            },
            &default_ctx(),
        );

        let project = vec![PathBuf::from("/work/otty")];
        assert_eq!(followed_folders(&split), project);
        assert_eq!(
            followed_folders(&shell),
            vec![PathBuf::from("/tmp/otty-first")]
        );
        assert_eq!(followed_folders(&agent), project);
    }

    /// A new theme colors every open terminal, and the panes split later.
    #[test]
    fn a_new_theme_colors_the_open_terminals() {
        let mut workspace = one_shell_tab();
        let palette = otty_ui_term::ColorPalette {
            background: String::from("#123456"),
            ..otty_ui_term::ColorPalette::default()
        };

        let outcomes = workspace.reduce(
            TerminalIntent::ApplyTheme {
                palette: Box::new(palette),
            },
            &default_ctx(),
        );

        let tab = workspace.state.tab(1).expect("the tab stays open");
        assert_eq!(tab.theme_background(), "#123456");
        assert!(outcomes.is_empty());
    }

    /// The home folder comes with the tab: a folder under it is shown
    /// with `~`.
    #[test]
    fn a_folder_under_the_given_home_is_shown_with_a_tilde() {
        let mut workspace = shell_tab_with_home(Some("/home/ann"));

        let _ = workspace.report_folder(100, "/home/ann/projects");

        assert_eq!(
            focused_folder_path(&workspace).as_deref(),
            Some("~/projects")
        );
    }

    /// Without a home folder, the folder is shown in full.
    #[test]
    fn without_a_home_the_folder_is_shown_in_full() {
        let mut workspace = shell_tab_with_home(None);

        let _ = workspace.report_folder(100, "/home/ann/projects");

        assert_eq!(
            focused_folder_path(&workspace).as_deref(),
            Some("/home/ann/projects")
        );
    }

    /// A full-screen program on the alternate screen, such as `vim`, after
    /// the command blocks `ids`.
    fn full_screen_program_after(ids: &[&str]) -> SnapshotOwned {
        let size = otty_libterm::TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 40,
            rows: 10,
        };
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks = command_history(ids).blocks;

        snapshot
    }

    /// Make terminal `terminal_id` show `snapshot`; the workspace must be
    /// [`Workspace::scripted`].
    fn show(
        workspace: &mut Workspace,
        terminal_id: u64,
        snapshot: SnapshotOwned,
    ) {
        workspace.terminals.report(terminal_id, snapshot);
        let _ = workspace.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::ContentSync {
                id: terminal_id,
            }),
            &default_ctx(),
        );
    }

    /// The selected block of tab 1: its terminal and its key.
    fn selected_block(workspace: &Workspace) -> Option<(u64, BlockKey)> {
        workspace
            .state
            .tab(1)
            .and_then(|tab| tab.selected_block())
            .map(|selected| (selected.terminal_id(), selected.block_key()))
    }

    /// The widget id of terminal `terminal_id` in tab 1.
    fn widget_id_of(
        workspace: &Workspace,
        terminal_id: u64,
    ) -> iced::widget::Id {
        workspace
            .state
            .tab(1)
            .and_then(|tab| tab.terminals().get(&terminal_id))
            .map(|entry| entry.terminal().widget_id().clone())
            .expect("the tab has the terminal")
    }

    /// Whether `outcomes` select the block named `name` in the widget
    /// `terminal` and then bring it into sight there.
    fn selects_and_brings_into_sight(
        outcomes: &[TerminalOutcome],
        terminal: &iced::widget::Id,
        name: &str,
    ) -> bool {
        let block_key = key_of(name);
        let position = |wanted: &dyn Fn(&BlockCommand) -> bool| {
            outcomes.iter().position(|outcome| {
                matches!(
                    outcome,
                    TerminalOutcome::RunBlockCommand { terminal: on, command }
                        if on == terminal && wanted(command)
                )
            })
        };
        let selected = position(
            &|command| matches!(command, BlockCommand::Select(key) if *key == block_key),
        );
        let in_sight = position(
            &|command| matches!(command, BlockCommand::ScrollTo(key) if *key == block_key),
        );

        matches!((selected, in_sight), (Some(select), Some(scroll)) if select < scroll)
    }

    /// Tab 1 whose pane, terminal 100, shows six finished commands above
    /// the prompt.
    fn six_commands() -> Workspace {
        let mut workspace = one_scripted_tab();
        show(
            &mut workspace,
            100,
            command_history(&[
                "cmd-1", "cmd-2", "cmd-3", "cmd-4", "cmd-5", "cmd-6",
            ]),
        );

        workspace
    }

    #[test]
    fn the_previous_block_key_with_nothing_selected_selects_the_latest_command_block_and_brings_it_into_sight()
     {
        let mut workspace = six_commands();
        let widget_id = widget_id_of(&workspace, 100);

        let outcomes = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), Some((100, key_of("cmd-6"))));
        assert!(
            selects_and_brings_into_sight(&outcomes, &widget_id, "cmd-6"),
            "got {outcomes:?}"
        );
    }

    #[test]
    fn the_next_block_key_with_nothing_selected_does_nothing() {
        let mut workspace = six_commands();

        let outcomes =
            workspace.reduce(TerminalIntent::SelectNextBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), None);
        assert!(outcomes.is_empty(), "got {outcomes:?}");
    }

    /// Two presses up and one down select the latest block, the one before
    /// it, then the latest again; the prompt is never selected.
    #[test]
    fn the_block_keys_move_the_selection_by_one_command_block() {
        let mut workspace = six_commands();
        let widget_id = widget_id_of(&workspace, 100);

        let _latest = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());
        let back = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());
        let after_back = selected_block(&workspace);
        let forward =
            workspace.reduce(TerminalIntent::SelectNextBlock, &default_ctx());

        assert_eq!(after_back, Some((100, key_of("cmd-5"))));
        assert!(selects_and_brings_into_sight(&back, &widget_id, "cmd-5"));
        assert_eq!(selected_block(&workspace), Some((100, key_of("cmd-6"))));
        assert!(selects_and_brings_into_sight(&forward, &widget_id, "cmd-6"));
    }

    /// The block keys move from a block selected with a click too, as the
    /// tab keeps one selection.
    #[test]
    fn the_block_keys_move_from_a_block_selected_with_a_click() {
        let mut workspace = six_commands();
        let _ = workspace.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::BlockSelected {
                id: 100,
                block_key: key_of("cmd-3"),
            }),
            &default_ctx(),
        );

        let _ =
            workspace.reduce(TerminalIntent::SelectNextBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), Some((100, key_of("cmd-4"))));
    }

    #[test]
    fn the_previous_block_key_at_the_first_block_and_the_next_at_the_last_change_nothing()
     {
        let mut workspace = one_scripted_tab();
        show(&mut workspace, 100, command_history(&["cmd-1"]));
        let _ = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());

        let up = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());
        let down =
            workspace.reduce(TerminalIntent::SelectNextBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), Some((100, key_of("cmd-1"))));
        assert!(up.is_empty(), "got {up:?}");
        assert!(down.is_empty(), "got {down:?}");
    }

    /// A full-screen program has no blocks: the keys reach it, and no
    /// block is selected even if they come another way.
    #[test]
    fn the_block_keys_select_nothing_while_a_full_screen_program_runs() {
        let mut workspace = one_scripted_tab();
        show(
            &mut workspace,
            100,
            full_screen_program_after(&["cmd-1", "cmd-2"]),
        );

        let outcomes = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), None);
        assert!(outcomes.is_empty(), "got {outcomes:?}");
    }

    /// The keys act on the focused pane. A block selected in another pane
    /// is cleared there, as the tab keeps one selection.
    #[test]
    fn the_block_keys_act_on_the_focused_pane_and_clear_the_selection_in_the_other()
     {
        let mut workspace = split_scripted_tab();
        show(&mut workspace, 100, command_history(&["left-1", "left-2"]));
        show(
            &mut workspace,
            101,
            command_history(&["right-1", "right-2"]),
        );
        let _ = workspace.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::BlockSelected {
                id: 100,
                block_key: key_of("left-1"),
            }),
            &default_ctx(),
        );
        let left = widget_id_of(&workspace, 100);
        let right = widget_id_of(&workspace, 101);

        let outcomes = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());

        assert_eq!(selected_block(&workspace), Some((101, key_of("right-2"))));
        assert!(selects_and_brings_into_sight(&outcomes, &right, "right-2"));
        assert!(
            outcomes.iter().any(|outcome| matches!(
                outcome,
                TerminalOutcome::RunBlockCommand {
                    terminal,
                    command: BlockCommand::ClearSelection,
                } if *terminal == left
            )),
            "got {outcomes:?}"
        );
    }

    #[test]
    fn the_block_keys_without_a_terminal_tab_in_front_do_nothing() {
        let mut workspace = six_commands();
        let _ = workspace
            .reduce(TerminalIntent::TabActivated { tab_id: 9 }, &default_ctx());

        let outcomes = workspace
            .reduce(TerminalIntent::SelectPreviousBlock, &default_ctx());

        assert!(outcomes.is_empty(), "got {outcomes:?}");
    }
}
