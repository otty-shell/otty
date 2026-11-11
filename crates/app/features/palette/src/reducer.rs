use otty_app_domain::commands::AppCommand;
use otty_app_domain::mode::Mode;
use otty_ui_kit::components::command_palette::PaletteAction;

use super::ctx::PaletteCtx;
use super::event::{PaletteIntent, PaletteOutcome};
use super::items::palette_items;
use super::state::PaletteState;
use super::types::PaletteTarget;

/// Reduce a palette intent into state changes and return what happened.
/// The palette only changes its own state: picking an entry closes it and
/// reports the pick for the binary to run.
pub(super) fn reduce(
    state: &mut PaletteState,
    intent: PaletteIntent,
    ctx: &PaletteCtx<'_>,
) -> Vec<PaletteOutcome> {
    match intent {
        PaletteIntent::Open => {
            if !state.is_open() {
                state.open(palette_items(ctx));
            }
            focus_input(state)
        },
        PaletteIntent::Dismiss => dismiss(state),
        PaletteIntent::FocusInput => focus_input(state),
        PaletteIntent::Action(action) => reduce_action(state, action, ctx),
    }
}

/// Apply what the user did inside the open palette.
fn reduce_action(
    state: &mut PaletteState,
    action: PaletteAction,
    ctx: &PaletteCtx<'_>,
) -> Vec<PaletteOutcome> {
    if !state.is_open() {
        return Vec::new();
    }

    match action {
        PaletteAction::QueryChanged(query) => {
            state.change_query(palette_items(ctx), query);
            Vec::new()
        },
        PaletteAction::MoveUp => {
            state.move_up();
            Vec::new()
        },
        PaletteAction::MoveDown => {
            state.move_down();
            Vec::new()
        },
        PaletteAction::RunActive => {
            let target = state.active_target().cloned();
            pick(state, target, ctx)
        },
        PaletteAction::RunItem(index) => {
            let target = state.target_at(index).cloned();
            pick(state, target, ctx)
        },
        PaletteAction::Close => dismiss(state),
    }
}

/// Close the palette and report `target` as picked; nothing happens when
/// there is no entry to pick. A command listed earlier that can no longer
/// run (for example a split whose tab closed meanwhile) only closes the
/// palette: the palette is where a command it sends is checked
/// ([`AppCommand::is_available`]).
fn pick(
    state: &mut PaletteState,
    target: Option<PaletteTarget>,
    ctx: &PaletteCtx<'_>,
) -> Vec<PaletteOutcome> {
    let Some(target) = target else {
        return Vec::new();
    };

    state.close();

    match target {
        PaletteTarget::Session { tab_id } => {
            vec![PaletteOutcome::SessionPicked { tab_id }]
        },
        // The launch is still starting, so the active terminal gets back
        // the keyboard the palette input took.
        PaletteTarget::Launch { path } => vec![
            PaletteOutcome::LaunchPicked { path },
            PaletteOutcome::KeyboardReleased,
        ],
        PaletteTarget::Command(command) => pick_command(command, ctx),
    }
}

/// Report `command` as picked when it can run now. The commands that move
/// no keyboard focus themselves also release the keyboard the palette
/// input took.
fn pick_command(
    command: AppCommand,
    ctx: &PaletteCtx<'_>,
) -> Vec<PaletteOutcome> {
    if !command.is_available(&ctx.commands) {
        return Vec::new();
    }

    let mut outcomes = vec![PaletteOutcome::CommandPicked(command)];
    if moves_no_focus(command) {
        outcomes.push(PaletteOutcome::KeyboardReleased);
    }

    outcomes
}

/// Return `true` for the commands that leave the keyboard focus where it
/// is: the panel toggles, and Terminal mode, whose terminal comes back on
/// screen.
fn moves_no_focus(command: AppCommand) -> bool {
    matches!(
        command,
        AppCommand::ToggleLeftPanel
            | AppCommand::ToggleFilesPanel
            | AppCommand::ToggleFocusMode
            | AppCommand::SwitchMode(Mode::Terminal)
    )
}

/// Close the palette without picking anything; the input gives up the
/// keyboard.
fn dismiss(state: &mut PaletteState) -> Vec<PaletteOutcome> {
    if !state.close() {
        return Vec::new();
    }

    vec![PaletteOutcome::KeyboardReleased]
}

/// Ask for the keyboard focus on the palette input while the palette is
/// open.
fn focus_input(state: &PaletteState) -> Vec<PaletteOutcome> {
    if !state.is_open() {
        return Vec::new();
    }

    vec![PaletteOutcome::FocusInput]
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::{AppCommand, CommandCtx};
    use otty_app_domain::mode::Mode;
    use otty_app_i18n::{self as i18n, Key};
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::ctx::{PaletteLaunch, PaletteSession};
    use crate::event::PaletteOutcome;

    fn ctx_with_sessions(titles: &[&'static str]) -> PaletteCtx<'static> {
        let sessions = titles
            .iter()
            .enumerate()
            .map(|(index, title)| PaletteSession {
                tab_id: index as u64,
                title,
                machine: "fedora",
                target: TargetKind::Local,
                is_active: index == 0,
            })
            .collect();

        PaletteCtx {
            sessions,
            launches: vec![PaletteLaunch {
                path: vec![String::from("servers"), String::from("laptop")],
                title: "laptop",
                is_ssh: true,
            }],
            pinned_modes: Vec::new(),
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: true,
            },
        }
    }

    fn ctx() -> PaletteCtx<'static> {
        ctx_with_sessions(&["zsh", "htop"])
    }

    /// The same entries as [`ctx`] while Containers mode is on screen.
    fn containers_ctx() -> PaletteCtx<'static> {
        let ctx = ctx();

        PaletteCtx {
            commands: CommandCtx {
                is_terminal_mode: false,
                ..ctx.commands
            },
            ..ctx
        }
    }

    fn open_state() -> PaletteState {
        let mut state = PaletteState::default();
        let _ = reduce(&mut state, PaletteIntent::Open, &ctx());
        state
    }

    fn open_containers_state() -> PaletteState {
        let mut state = PaletteState::default();
        let _ = reduce(&mut state, PaletteIntent::Open, &containers_ctx());
        state
    }

    fn act_in_containers(
        state: &mut PaletteState,
        action: PaletteAction,
    ) -> Vec<PaletteOutcome> {
        reduce(state, PaletteIntent::Action(action), &containers_ctx())
    }

    fn listed_targets(state: &PaletteState) -> Vec<PaletteTarget> {
        state
            .list()
            .expect("palette should be open")
            .matches()
            .map(|(item, _)| item.key().clone())
            .collect()
    }

    fn listed_commands(state: &PaletteState) -> Vec<AppCommand> {
        listed_targets(state)
            .into_iter()
            .filter_map(|target| match target {
                PaletteTarget::Command(command) => Some(command),
                _ => None,
            })
            .collect()
    }

    fn act(
        state: &mut PaletteState,
        action: PaletteAction,
    ) -> Vec<PaletteOutcome> {
        reduce(state, PaletteIntent::Action(action), &ctx())
    }

    fn active_title(state: &PaletteState) -> Option<String> {
        let list = state.list()?;
        let index = list.active()?;

        list.matches()
            .nth(index)
            .map(|(item, _)| item.title().to_owned())
    }

    #[test]
    fn opening_lists_every_entry_with_the_first_active_and_focuses_the_input() {
        let mut state = PaletteState::default();

        let outcomes = reduce(&mut state, PaletteIntent::Open, &ctx());

        assert_eq!(outcomes, vec![PaletteOutcome::FocusInput]);
        let list = state.list().expect("palette should be open");
        assert_eq!(list.query(), "");
        assert_eq!(list.match_count(), palette_items(&ctx()).len());
        assert_eq!(list.active(), Some(0));
        assert_eq!(
            state.active_target(),
            Some(&PaletteTarget::Session { tab_id: 0 })
        );
    }

    #[test]
    fn opening_while_open_keeps_the_query_and_focuses_the_input_again() {
        let mut state = open_state();
        let _ = act(&mut state, PaletteAction::QueryChanged("ht".to_owned()));

        let outcomes = reduce(&mut state, PaletteIntent::Open, &ctx());

        assert_eq!(outcomes, vec![PaletteOutcome::FocusInput]);
        assert_eq!(state.list().map(|list| list.query()), Some("ht"));
    }

    #[test]
    fn typing_filters_the_entries() {
        let mut state = open_state();

        let outcomes =
            act(&mut state, PaletteAction::QueryChanged("lap".to_owned()));

        assert!(outcomes.is_empty());
        let list = state.list().expect("palette should stay open");
        assert_eq!(list.query(), "lap");
        assert_eq!(list.match_count(), 1);
        assert_eq!(
            state.active_target(),
            Some(&PaletteTarget::Launch {
                path: vec![String::from("servers"), String::from("laptop")],
            })
        );
    }

    #[test]
    fn typing_reads_the_entries_again_from_the_other_features() {
        let mut state = open_state();
        let newer = ctx_with_sessions(&["zsh", "htop", "vim notes"]);

        let _ = reduce(
            &mut state,
            PaletteIntent::Action(PaletteAction::QueryChanged(
                "vim".to_owned(),
            )),
            &newer,
        );

        assert_eq!(active_title(&state).as_deref(), Some("vim notes"));
    }

    #[test]
    fn arrows_move_the_active_entry() {
        let mut state = open_state();

        let down = act(&mut state, PaletteAction::MoveDown);
        let after_down = active_title(&state);
        let up = act(&mut state, PaletteAction::MoveUp);

        assert_eq!(after_down.as_deref(), Some("htop"));
        assert_eq!(active_title(&state).as_deref(), Some("zsh"));
        assert!(down.is_empty());
        assert!(up.is_empty());
    }

    #[test]
    fn enter_switches_to_the_active_session_and_closes_the_palette() {
        let mut state = open_state();
        let _ = act(&mut state, PaletteAction::MoveDown);

        let outcomes = act(&mut state, PaletteAction::RunActive);

        assert_eq!(outcomes, vec![PaletteOutcome::SessionPicked { tab_id: 1 }]);
        assert!(!state.is_open());
    }

    #[test]
    fn clicking_an_entry_picks_each_kind_of_item_and_closes_the_palette() {
        let laptop = vec![String::from("servers"), String::from("laptop")];
        let cases = [
            (0, vec![PaletteOutcome::SessionPicked { tab_id: 0 }]),
            (
                2,
                vec![
                    PaletteOutcome::LaunchPicked { path: laptop },
                    PaletteOutcome::KeyboardReleased,
                ],
            ),
            (
                3,
                vec![PaletteOutcome::CommandPicked(AppCommand::NewTerminal)],
            ),
            (
                5,
                vec![PaletteOutcome::CommandPicked(AppCommand::NewFolder)],
            ),
            (
                13,
                vec![
                    PaletteOutcome::CommandPicked(AppCommand::ToggleFocusMode),
                    PaletteOutcome::KeyboardReleased,
                ],
            ),
        ];

        for (index, expected) in cases {
            let mut state = open_state();

            let outcomes = act(&mut state, PaletteAction::RunItem(index));

            assert_eq!(outcomes, expected, "entry {index}");
            assert!(!state.is_open(), "entry {index} should close it");
        }
    }

    /// The panel toggles and Terminal mode move no keyboard focus
    /// themselves, so the active terminal gets back the keyboard the
    /// palette input took. Every other command runs alone.
    #[test]
    fn a_picked_command_releases_the_keyboard_only_when_it_moves_no_focus() {
        let releasing = [
            AppCommand::ToggleLeftPanel,
            AppCommand::ToggleFilesPanel,
            AppCommand::ToggleFocusMode,
            AppCommand::SwitchMode(Mode::Terminal),
        ];
        let commands = listed_commands(&open_state());

        for command in commands {
            let mut state = open_state();
            let index = listed_targets(&state)
                .iter()
                .position(|target| *target == PaletteTarget::Command(command))
                .expect("listed");

            let outcomes = act(&mut state, PaletteAction::RunItem(index));

            let mut expected = vec![PaletteOutcome::CommandPicked(command)];
            if releasing.contains(&command) {
                expected.push(PaletteOutcome::KeyboardReleased);
            }
            assert_eq!(outcomes, expected, "{command:?}");
        }
    }

    #[test]
    fn in_containers_mode_a_click_runs_the_entry_shown_at_that_place() {
        let cases = [
            (5, AppCommand::OpenSettings),
            (6, AppCommand::OpenNotifications),
            (7, AppCommand::ToggleLeftPanel),
            (8, AppCommand::ToggleFilesPanel),
            (9, AppCommand::ToggleFocusMode),
        ];

        for (index, expected) in cases {
            let mut state = open_containers_state();
            assert_eq!(
                listed_targets(&state)[index],
                PaletteTarget::Command(expected)
            );

            let outcomes =
                act_in_containers(&mut state, PaletteAction::RunItem(index));

            assert_eq!(
                outcomes.first(),
                Some(&PaletteOutcome::CommandPicked(expected)),
                "entry {index}"
            );
        }
    }

    #[test]
    fn in_containers_mode_arrows_and_enter_run_the_entry_they_reach() {
        let mut state = open_containers_state();

        for _ in 0..7 {
            let _ = act_in_containers(&mut state, PaletteAction::MoveDown);
        }
        let outcomes = act_in_containers(&mut state, PaletteAction::RunActive);

        assert_eq!(
            outcomes.first(),
            Some(&PaletteOutcome::CommandPicked(AppCommand::ToggleLeftPanel))
        );
    }

    #[test]
    fn in_containers_mode_typing_the_name_of_a_hidden_command_finds_nothing() {
        for key in [
            Key::PaletteSplitRight,
            Key::PaletteSplitDown,
            Key::CloseSession,
            Key::PaletteNewFolder,
        ] {
            let mut state = open_containers_state();

            let _ = act_in_containers(
                &mut state,
                PaletteAction::QueryChanged(i18n::t(key).to_owned()),
            );
            let outcomes =
                act_in_containers(&mut state, PaletteAction::RunActive);

            assert_eq!(state.list().map(|list| list.match_count()), Some(0));
            assert!(outcomes.is_empty());
            assert!(state.is_open());
        }
    }

    #[test]
    fn in_containers_mode_a_filtered_entry_runs_from_its_place_in_the_matches()
    {
        let mut state = open_containers_state();

        let _ = act_in_containers(
            &mut state,
            PaletteAction::QueryChanged(
                i18n::t(Key::WindowFocusMode).to_owned(),
            ),
        );
        let outcomes = act_in_containers(&mut state, PaletteAction::RunItem(0));

        assert_eq!(
            outcomes.first(),
            Some(&PaletteOutcome::CommandPicked(AppCommand::ToggleFocusMode))
        );
    }

    #[test]
    fn typing_lists_the_commands_of_the_mode_on_screen_now() {
        let mut state = open_state();
        let in_terminal = listed_commands(&state);

        let _ = act_in_containers(
            &mut state,
            PaletteAction::QueryChanged(String::new()),
        );

        let in_containers = listed_commands(&state);
        assert!(in_terminal.contains(&AppCommand::SplitRight));
        assert!(in_terminal.contains(&AppCommand::NewFolder));
        for command in [
            AppCommand::SplitRight,
            AppCommand::CloseSession,
            AppCommand::NewFolder,
        ] {
            assert!(!in_containers.contains(&command), "{command:?}");
        }
    }

    #[test]
    fn an_entry_that_can_no_longer_run_closes_the_palette_and_runs_nothing() {
        // Listed in Terminal mode, chosen after a switch to Containers.
        for command in [
            AppCommand::CloseSession,
            AppCommand::SplitRight,
            AppCommand::SplitDown,
            AppCommand::NewFolder,
        ] {
            let mut state = open_state();
            let index = listed_targets(&state)
                .iter()
                .position(|target| *target == PaletteTarget::Command(command))
                .expect("listed in Terminal mode");

            let outcomes =
                act_in_containers(&mut state, PaletteAction::RunItem(index));

            assert!(outcomes.is_empty(), "{command:?}");
            assert!(!state.is_open(), "{command:?}");
        }
    }

    #[test]
    fn enter_with_nothing_matching_runs_nothing_and_stays_open() {
        let mut state = open_state();
        let _ = act(&mut state, PaletteAction::QueryChanged("zzz".to_owned()));

        let outcomes = act(&mut state, PaletteAction::RunActive);

        assert!(outcomes.is_empty());
        assert!(state.is_open());
    }

    #[test]
    fn clicking_past_the_last_match_runs_nothing() {
        let mut state = open_state();

        let outcomes = act(&mut state, PaletteAction::RunItem(99));

        assert!(outcomes.is_empty());
        assert!(state.is_open());
    }

    #[test]
    fn esc_closes_the_palette_and_releases_the_keyboard() {
        let mut state = open_state();

        let outcomes = act(&mut state, PaletteAction::Close);

        assert_eq!(outcomes, vec![PaletteOutcome::KeyboardReleased]);
        assert!(!state.is_open());
    }

    #[test]
    fn a_click_outside_closes_the_palette_and_releases_the_keyboard() {
        let mut state = open_state();

        let outcomes = reduce(&mut state, PaletteIntent::Dismiss, &ctx());

        assert_eq!(outcomes, vec![PaletteOutcome::KeyboardReleased]);
        assert!(!state.is_open());
    }

    #[test]
    fn closing_a_closed_palette_reports_nothing() {
        let mut state = PaletteState::default();

        let dismissed = reduce(&mut state, PaletteIntent::Dismiss, &ctx());
        let closed = act(&mut state, PaletteAction::Close);

        assert!(dismissed.is_empty());
        assert!(closed.is_empty());
    }

    #[test]
    fn palette_actions_while_closed_are_ignored() {
        let actions = [
            PaletteAction::QueryChanged("zsh".to_owned()),
            PaletteAction::MoveDown,
            PaletteAction::MoveUp,
            PaletteAction::RunActive,
            PaletteAction::RunItem(0),
        ];

        for action in actions {
            let mut state = PaletteState::default();

            let outcomes = act(&mut state, action.clone());

            assert!(outcomes.is_empty(), "{action:?}");
            assert!(!state.is_open(), "{action:?}");
        }
    }

    #[test]
    fn focus_input_focuses_only_an_open_palette() {
        let mut closed = PaletteState::default();
        let mut open = open_state();

        let closed_outcomes =
            reduce(&mut closed, PaletteIntent::FocusInput, &ctx());
        let open_outcomes =
            reduce(&mut open, PaletteIntent::FocusInput, &ctx());

        assert!(closed_outcomes.is_empty());
        assert_eq!(open_outcomes, vec![PaletteOutcome::FocusInput]);
    }
}
