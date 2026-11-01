use iced::Task;
use iced::widget::operation;
use otty_ui_kit::components::command_palette::{PaletteAction, input_id};

use super::event::{PaletteEffect, PaletteEvent, PaletteIntent};
use super::items::palette_items;
use super::state::PaletteState;
use super::types::PaletteTarget;
use crate::widgets::quick_launch::model::LaunchViewModel;
use crate::widgets::tabs::model::TabsViewModel;

/// What the palette lists, read from the other widgets' view models when
/// it opens and on every query change.
pub(crate) struct PaletteCtx<'a> {
    /// Open tabs grouped by machine.
    pub(crate) sessions: TabsViewModel<'a>,
    /// Every launch of the launch tree.
    pub(crate) launches: Vec<LaunchViewModel<'a>>,
    /// The active tab is a terminal with a focused pane to split.
    pub(crate) can_split_pane: bool,
}

/// Reduce a palette intent into state changes and effects. The palette
/// only changes its own state: running an entry closes it and emits
/// [`PaletteEffect::Run`] for the app to route.
pub(crate) fn reduce(
    state: &mut PaletteState,
    intent: PaletteIntent,
    ctx: &PaletteCtx<'_>,
) -> Task<PaletteEvent> {
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
) -> Task<PaletteEvent> {
    if !state.is_open() {
        return Task::none();
    }

    match action {
        PaletteAction::QueryChanged(query) => {
            state.change_query(palette_items(ctx), query);
            Task::none()
        },
        PaletteAction::MoveUp => {
            state.move_up();
            Task::none()
        },
        PaletteAction::MoveDown => {
            state.move_down();
            Task::none()
        },
        PaletteAction::RunActive => {
            let target = state.active_target().cloned();
            run(state, target)
        },
        PaletteAction::RunItem(index) => {
            let target = state.target_at(index).cloned();
            run(state, target)
        },
        PaletteAction::Close => dismiss(state),
    }
}

/// Close the palette and hand `target` to the app; nothing happens when
/// there is no entry to run.
fn run(
    state: &mut PaletteState,
    target: Option<PaletteTarget>,
) -> Task<PaletteEvent> {
    let Some(target) = target else {
        return Task::none();
    };

    state.close();

    Task::done(PaletteEvent::Effect(PaletteEffect::Run(target)))
}

/// Close the palette without running anything.
fn dismiss(state: &mut PaletteState) -> Task<PaletteEvent> {
    if !state.close() {
        return Task::none();
    }

    Task::done(PaletteEvent::Effect(PaletteEffect::Dismissed))
}

/// Give the palette input keyboard focus while the palette is open. Moving
/// focus there also takes it from the terminal panes.
fn focus_input(state: &PaletteState) -> Task<PaletteEvent> {
    if !state.is_open() {
        return Task::none();
    }

    operation::focus(input_id())
}

#[cfg(test)]
mod tests {
    use iced::futures::StreamExt;
    use iced::futures::executor::block_on;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::task_outputs::collect_outputs;
    use crate::widgets::tabs::model::{
        SessionGroupViewModel, SessionViewModel,
    };

    fn ctx_with_sessions(titles: &[&'static str]) -> PaletteCtx<'static> {
        let sessions = titles
            .iter()
            .enumerate()
            .map(|(index, title)| SessionViewModel {
                tab_id: index as u64,
                title,
                folder: None,
                state: None,
                is_active: index == 0,
            })
            .collect::<Vec<_>>();
        let session_count = sessions.len();

        PaletteCtx {
            sessions: TabsViewModel {
                groups: vec![SessionGroupViewModel {
                    name: "fedora",
                    target: TargetKind::Local,
                    sessions,
                }],
                session_count,
            },
            launches: vec![LaunchViewModel {
                path: vec![String::from("servers"), String::from("laptop")],
                title: "laptop",
                is_ssh: true,
            }],
            can_split_pane: true,
        }
    }

    fn ctx() -> PaletteCtx<'static> {
        ctx_with_sessions(&["zsh", "htop"])
    }

    fn open_state() -> PaletteState {
        let mut state = PaletteState::default();
        let _ = reduce(&mut state, PaletteIntent::Open, &ctx());
        state
    }

    fn act(
        state: &mut PaletteState,
        action: PaletteAction,
    ) -> Vec<PaletteEvent> {
        collect_outputs(reduce(state, PaletteIntent::Action(action), &ctx()))
    }

    fn effects(events: Vec<PaletteEvent>) -> Vec<PaletteEffect> {
        events
            .into_iter()
            .filter_map(|event| match event {
                PaletteEvent::Effect(effect) => Some(effect),
                PaletteEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Return whether `task` asks for a widget operation, such as moving
    /// keyboard focus.
    fn runs_widget_operation(task: Task<PaletteEvent>) -> bool {
        let Some(stream) = into_stream(task) else {
            return false;
        };
        let actions: Vec<Action<PaletteEvent>> = block_on(stream.collect());

        actions
            .iter()
            .any(|action| matches!(action, Action::Widget(_)))
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

        let task = reduce(&mut state, PaletteIntent::Open, &ctx());

        assert!(runs_widget_operation(task));
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

        let task = reduce(&mut state, PaletteIntent::Open, &ctx());

        assert!(runs_widget_operation(task));
        assert_eq!(state.list().map(|list| list.query()), Some("ht"));
    }

    #[test]
    fn typing_filters_the_entries() {
        let mut state = open_state();

        let events =
            act(&mut state, PaletteAction::QueryChanged("lap".to_owned()));

        assert!(events.is_empty());
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
    fn typing_reads_the_entries_again_from_the_other_widgets() {
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

        let _ = act(&mut state, PaletteAction::MoveDown);
        assert_eq!(active_title(&state).as_deref(), Some("htop"));

        let _ = act(&mut state, PaletteAction::MoveUp);
        assert_eq!(active_title(&state).as_deref(), Some("zsh"));
    }

    #[test]
    fn enter_runs_the_active_entry_and_closes_the_palette() {
        let mut state = open_state();
        let _ = act(&mut state, PaletteAction::MoveDown);

        let events = act(&mut state, PaletteAction::RunActive);

        assert_eq!(
            effects(events),
            vec![PaletteEffect::Run(PaletteTarget::Session { tab_id: 1 })]
        );
        assert!(!state.is_open());
    }

    #[test]
    fn clicking_an_entry_runs_each_kind_of_item_and_closes_the_palette() {
        let laptop = PaletteTarget::Launch {
            path: vec![String::from("servers"), String::from("laptop")],
        };
        let cases = [
            (0, PaletteTarget::Session { tab_id: 0 }),
            (2, laptop),
            (3, PaletteTarget::NewTerminal),
            (5, PaletteTarget::NewFolder),
            (11, PaletteTarget::ToggleFocusMode),
        ];

        for (index, expected) in cases {
            let mut state = open_state();

            let events = act(&mut state, PaletteAction::RunItem(index));

            assert_eq!(
                effects(events),
                vec![PaletteEffect::Run(expected.clone())],
                "entry {index}"
            );
            assert!(!state.is_open(), "entry {index} should close it");
        }
    }

    #[test]
    fn enter_with_nothing_matching_runs_nothing_and_stays_open() {
        let mut state = open_state();
        let _ = act(&mut state, PaletteAction::QueryChanged("zzz".to_owned()));

        let events = act(&mut state, PaletteAction::RunActive);

        assert!(events.is_empty());
        assert!(state.is_open());
    }

    #[test]
    fn clicking_past_the_last_match_runs_nothing() {
        let mut state = open_state();

        let events = act(&mut state, PaletteAction::RunItem(99));

        assert!(events.is_empty());
        assert!(state.is_open());
    }

    #[test]
    fn esc_closes_the_palette_without_running_anything() {
        let mut state = open_state();

        let events = act(&mut state, PaletteAction::Close);

        assert_eq!(effects(events), vec![PaletteEffect::Dismissed]);
        assert!(!state.is_open());
    }

    #[test]
    fn a_click_outside_closes_the_palette() {
        let mut state = open_state();

        let events =
            collect_outputs(reduce(&mut state, PaletteIntent::Dismiss, &ctx()));

        assert_eq!(effects(events), vec![PaletteEffect::Dismissed]);
        assert!(!state.is_open());
    }

    #[test]
    fn closing_a_closed_palette_emits_nothing() {
        let mut state = PaletteState::default();

        let dismissed =
            collect_outputs(reduce(&mut state, PaletteIntent::Dismiss, &ctx()));
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

            let events = act(&mut state, action.clone());

            assert!(events.is_empty(), "{action:?}");
            assert!(!state.is_open(), "{action:?}");
        }
    }

    #[test]
    fn focus_input_focuses_only_an_open_palette() {
        let mut closed = PaletteState::default();
        let mut open = open_state();

        let closed_task =
            reduce(&mut closed, PaletteIntent::FocusInput, &ctx());
        let open_task = reduce(&mut open, PaletteIntent::FocusInput, &ctx());

        assert!(!runs_widget_operation(closed_task));
        assert!(runs_widget_operation(open_task));
    }
}
