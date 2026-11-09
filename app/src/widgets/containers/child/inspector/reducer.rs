#![forbid(clippy::disallowed_methods)]
//! What the inspector does (spec Decision 12): it follows the logs of the
//! selected container while the Logs tab is on screen, loads the inspect
//! JSON of the selected row once per selection while the Inspect tab is on
//! screen, keeps the inspect tree up to date and hands a pressed link to the
//! parent.

use iced::Task;
use otty_containers::{EngineAddress, EngineInfo};

use super::super::super::links::LinkTarget;
use super::super::super::types::{InspectorTab, ListRow, OpenList};
use super::event::{InspectorEffect, InspectorIntent};
use super::inspect_links::InspectLinks;
use super::inspect_tree::inspect_tree;
use super::state::InspectorState;
use super::types::{InspectTarget, LogTarget};

/// What the inspector reads from the parent widget.
#[derive(Debug)]
pub(crate) struct InspectorCtx<'a> {
    /// Whether the inspector is on screen: Containers mode is active, the
    /// right panel is shown and the Run form, which shows the image panel in
    /// its place, is closed.
    pub(crate) is_on_screen: bool,
    /// The open tab; the parent keeps it.
    pub(crate) tab: InspectorTab,
    /// Goes up whenever a list of any engine may have changed; the inspect
    /// tree is built again when it moves, because its links point into the
    /// lists.
    pub(crate) lists_version: u64,
    /// The list the main area shows; `None` without engines.
    pub(crate) open: Option<OpenList<'a>>,
}

/// What the inspector tells the parent widget, which acts on it in the same
/// update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InspectorOutcome {
    /// A link was pressed in the Overview or in the Inspect tree: follow it.
    FollowLink(LinkTarget),
}

/// Reduce an intent of the inspector: follow the selection, take the lines
/// of the followed logs and the inspect answer, and hand a pressed link to
/// the parent.
pub(crate) fn reduce(
    state: &mut InspectorState,
    intent: InspectorIntent,
    ctx: &InspectorCtx<'_>,
) -> (Task<InspectorEffect>, Option<InspectorOutcome>) {
    match intent {
        InspectorIntent::FollowSelection => {
            (follow_selection(state, ctx), None)
        },
        InspectorIntent::EngineReconnected(engine) => {
            (check_open_logs(state, &engine), None)
        },
        InspectorIntent::ContainerStarted { engine, id } => {
            state.restart_logs(&engine, &id);
            (Task::none(), None)
        },
        InspectorIntent::LogLines {
            target,
            lines,
            position,
        } => {
            if !state.push_log_lines(&target, lines, position) {
                log::debug!("containers log lines of an unfollowed container");
            }
            (Task::none(), None)
        },
        InspectorIntent::LogRunRead { target, run } => {
            if !state.set_log_run(&target, run) {
                log::debug!("containers log run of an unfollowed container");
            }
            (Task::none(), None)
        },
        InspectorIntent::LogRunChecked { target, result } => {
            let run_now = result
                .inspect_err(|err| log::debug!("containers log run: {err}"))
                .ok();

            if !state.settle_logs_after_reconnect(&target, run_now) {
                log::debug!(
                    "containers log run check of an older stream dropped"
                );
            }
            (Task::none(), None)
        },
        InspectorIntent::LogsFailed { target, error } => {
            if !state.fail_logs(&target, error) {
                log::debug!("containers log error of an old stream dropped");
            }
            (Task::none(), None)
        },
        InspectorIntent::LogAction(action) => {
            state.perform_log_action(action);
            (Task::none(), None)
        },
        InspectorIntent::LogFollowToggled(is_following) => {
            state.set_log_following(is_following);
            (Task::none(), None)
        },
        InspectorIntent::InspectLoaded { target, result } => {
            if !state.finish_inspect(&target, result) {
                log::debug!("containers inspect of a row no longer shown");
            }
            (Task::none(), None)
        },
        InspectorIntent::InspectAction(action) => {
            state.perform_inspect_action(action);
            (Task::none(), None)
        },
        InspectorIntent::InspectSectionToggled(title) => {
            state.toggle_inspect_section(&title);
            (Task::none(), None)
        },
        InspectorIntent::InspectShowEmptyToggled(is_on) => {
            state.set_shows_empty(is_on);
            (Task::none(), None)
        },
        InspectorIntent::InspectRawJsonToggled(is_on) => {
            state.set_raw_json(is_on);
            (Task::none(), None)
        },
        InspectorIntent::CopyInspectJson => {
            let copy = match state.inspect_json_text() {
                Some(json) => {
                    Task::done(InspectorEffect::CopyToClipboard(json))
                },
                None => Task::none(),
            };
            (copy, None)
        },
        InspectorIntent::LinkPressed(target) => {
            (Task::none(), Some(InspectorOutcome::FollowLink(target)))
        },
    }
}

/// Make the inspector match the parent after one of its intents: follow the
/// logs of the selected container only while the Logs tab is on screen, and
/// load the inspect JSON once per selection while the Inspect tab is on
/// screen. A tab, selection or mode change stops the old logs, and so does
/// hiding the inspector; a selection change forgets the old JSON and its open
/// sections.
fn follow_selection(
    state: &mut InspectorState,
    ctx: &InspectorCtx<'_>,
) -> Task<InspectorEffect> {
    state.follow_logs(wanted_log_target(ctx));

    let selected = selected_inspect_target(ctx);
    state.forget_inspect_unless(selected.as_ref());
    refresh_inspect_tree(state, ctx);

    let Some((engine, target)) = wanted_inspect_target(ctx) else {
        return Task::none();
    };
    if !state.start_inspect(target.clone()) {
        return Task::none();
    }

    Task::done(InspectorEffect::LoadInspect { engine, target })
}

/// Return the container whose logs should be followed now: the Logs tab is
/// on screen and a container is selected.
fn wanted_log_target(ctx: &InspectorCtx<'_>) -> Option<LogTarget> {
    if !ctx.is_on_screen || ctx.tab != InspectorTab::Logs {
        return None;
    }

    let open = ctx.open?;
    let ListRow::Container(container) = open.selected_row()? else {
        return None;
    };

    Some(LogTarget::new(open.engine.clone(), container.id()))
}

/// Return the row whose inspect JSON should be shown now, with its engine:
/// the Inspect tab is on screen and a row is selected.
fn wanted_inspect_target(
    ctx: &InspectorCtx<'_>,
) -> Option<(EngineInfo, InspectTarget)> {
    if !ctx.is_on_screen || ctx.tab != InspectorTab::Inspect {
        return None;
    }

    let open = ctx.open?;
    let target = selected_inspect_target(ctx)?;

    Some((open.engine.clone(), target))
}

/// Return the selected row as an inspect target, on any tab and in any
/// mode; `None` when nothing is selected.
fn selected_inspect_target(ctx: &InspectorCtx<'_>) -> Option<InspectTarget> {
    let open = ctx.open?;
    let row = open.selected_row()?;

    Some(InspectTarget::new(
        open.engine.address().clone(),
        row.resource(),
        row.id(),
    ))
}

/// Build the inspect tree again when what it comes from has changed: the
/// answer, the engine's lists (its links point into them) or the Show empty
/// switch. Only the tree of the open Inspect tab is built; the view only
/// reads the result.
fn refresh_inspect_tree(state: &mut InspectorState, ctx: &InspectorCtx<'_>) {
    if ctx.tab != InspectorTab::Inspect {
        return;
    }
    let Some(value) = state.answer_of_old_tree(ctx.lists_version) else {
        return;
    };
    let Some(open) = ctx.open else {
        return;
    };
    let Some(row) = open.selected_row() else {
        return;
    };

    let shown_container = match row {
        ListRow::Container(container) => Some(container),
        _ => None,
    };
    let links = InspectLinks {
        resource: row.resource(),
        shown_container,
        containers: open.containers,
        images: open.images,
        volumes: open.volumes,
        networks: open.networks,
    };
    let rows = inspect_tree(value, state.shows_empty(), &links);

    state.keep_inspect_tree(rows, ctx.lists_version);
}

/// The engine at `address` reconnected, so a start of the container whose
/// logs are open may have been missed: ask what became of it. Logs are open
/// only while the inspector is on screen.
fn check_open_logs(
    state: &InspectorState,
    address: &EngineAddress,
) -> Task<InspectorEffect> {
    let target = state
        .logs()
        .map(|logs| logs.target())
        .filter(|target| target.is_on_engine(address));

    match target {
        Some(target) => {
            Task::done(InspectorEffect::CheckLogRun(target.clone()))
        },
        None => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::text_editor::Action;
    use otty_containers::{
        Container, ContainerRun, EngineKind, LogPosition, MAX_LOG_LINE_BYTES,
        Resources, fixtures,
    };
    use otty_ui_kit::components::data_tree::DataRow;
    use otty_ui_kit::components::key_value_list::ValuePart;
    use otty_ui_kit::components::log_view::LogViewState;

    use super::super::inspect_tree::InspectTree;
    use super::super::model::{
        InspectContent, InspectorTabsViewModel, TabContent,
    };
    use super::super::state::{InspectLoad, MAX_LOG_LINES, MAX_LOG_TEXT_BYTES};
    use super::super::types::LogRead;
    use super::*;
    use crate::i18n::{self, Key};
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::child::run::event::RunIntent;
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::reducer::reduce as reduce_containers;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::table_search::keys;
    use crate::widgets::containers::types::{
        ContainersError, LoadedList, LoadedLists, Place, ResourceKind,
    };

    /// Reduce `intent` in the child widget alone; the effects it asks for
    /// and what it tells the parent.
    fn reduce_child(
        state: &mut InspectorState,
        intent: InspectorIntent,
        ctx: &InspectorCtx<'_>,
    ) -> (Vec<InspectorEffect>, Option<InspectorOutcome>) {
        let (effects, outcome) = reduce(state, intent, ctx);

        (collect_outputs(effects), outcome)
    }

    /// What the parent lends the child widget: the `resource` list of
    /// `engine` open with the row `selected`, on `tab`, on screen or not.
    fn ctx<'a>(
        resources: &'a Resources,
        engine: &'a EngineInfo,
        resource: ResourceKind,
        selected: Option<&'a str>,
        tab: InspectorTab,
        is_on_screen: bool,
    ) -> InspectorCtx<'a> {
        InspectorCtx {
            is_on_screen,
            tab,
            lists_version: 1,
            open: Some(OpenList {
                engine,
                resource,
                containers: resources.containers(),
                images: resources.images(),
                volumes: resources.volumes(),
                networks: resources.networks(),
                selected_id: selected,
            }),
        }
    }

    /// The id of the container whose logs the child widget follows.
    fn followed_in_part(state: &InspectorState) -> Option<&str> {
        state.logs().map(|logs| logs.target().container_id())
    }

    #[test]
    fn following_the_selection_on_the_logs_tab_follows_the_selected_container()
    {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let id = resources.containers()[1].id();
        let mut state = InspectorState::default();
        let on_logs = ctx(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(id),
            InspectorTab::Logs,
            true,
        );

        let (effects, outcome) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_logs,
        );

        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(outcome, None);
        assert_eq!(
            state.logs().map(|logs| logs.target().clone()),
            Some(LogTarget::new(docker(), id))
        );
    }

    #[test]
    fn following_the_selection_off_screen_follows_and_loads_nothing() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let id = resources.containers()[0].id();
        let mut state = InspectorState::default();

        for tab in [InspectorTab::Logs, InspectorTab::Inspect] {
            let hidden = ctx(
                &resources,
                &engine,
                ResourceKind::Containers,
                Some(id),
                tab,
                false,
            );

            let (effects, _) = reduce_child(
                &mut state,
                InspectorIntent::FollowSelection,
                &hidden,
            );

            assert!(effects.is_empty(), "{tab:?}: {effects:?}");
            assert_eq!(followed_in_part(&state), None, "{tab:?}");
            assert!(state.inspect().is_none(), "{tab:?}");
        }
    }

    #[test]
    fn following_the_selection_on_the_overview_follows_and_loads_nothing() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let id = resources.containers()[0].id();
        let mut state = InspectorState::default();
        let on_overview = ctx(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(id),
            InspectorTab::Overview,
            true,
        );

        let (effects, _) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_overview,
        );

        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(followed_in_part(&state), None);
        assert!(state.inspect().is_none());
    }

    #[test]
    fn following_the_selection_on_the_inspect_tab_loads_the_selected_row_once()
    {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let volume = resources.volumes()[0].name();
        let mut state = InspectorState::default();
        let on_inspect = ctx(
            &resources,
            &engine,
            ResourceKind::Volumes,
            Some(volume),
            InspectorTab::Inspect,
            true,
        );

        let (first, _) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_inspect,
        );
        let (second, _) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_inspect,
        );

        assert_eq!(
            first,
            vec![InspectorEffect::LoadInspect {
                engine: docker(),
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Volumes,
                    volume,
                ),
            }]
        );
        assert!(second.is_empty(), "{second:?}");
        assert_eq!(followed_in_part(&state), None);
    }

    #[test]
    fn following_another_selection_drops_what_the_old_one_showed() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let first = resources.containers()[0].id();
        let second = resources.containers()[1].id();
        let mut state = InspectorState::default();
        let on_first = ctx(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(first),
            InspectorTab::Inspect,
            true,
        );
        let _ = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_first,
        );
        let on_second = ctx(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(second),
            InspectorTab::Logs,
            true,
        );

        let (effects, _) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &on_second,
        );

        assert!(effects.is_empty(), "{effects:?}");
        assert!(state.inspect().is_none());
        assert_eq!(followed_in_part(&state), Some(second));
    }

    #[test]
    fn following_the_selection_without_an_open_list_follows_nothing() {
        let mut state = InspectorState::default();
        let nothing_open = InspectorCtx {
            is_on_screen: true,
            tab: InspectorTab::Logs,
            lists_version: 0,
            open: None,
        };

        let (effects, outcome) = reduce_child(
            &mut state,
            InspectorIntent::FollowSelection,
            &nothing_open,
        );

        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(outcome, None);
        assert_eq!(followed_in_part(&state), None);
    }

    #[test]
    fn a_pressed_link_goes_to_the_parent_to_follow() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = docker();
        let id = resources.containers()[0].id();
        let on_overview = ctx(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(id),
            InspectorTab::Overview,
            true,
        );
        let targets = [
            LinkTarget::Url(String::from("http://localhost:18081")),
            LinkTarget::Item(ResourceKind::Images, String::from("sha256:a7db")),
            LinkTarget::ComposeProject(String::from("otty-fx")),
        ];

        for target in targets {
            let mut state = InspectorState::default();

            let (effects, outcome) = reduce_child(
                &mut state,
                InspectorIntent::LinkPressed(target.clone()),
                &on_overview,
            );

            assert!(effects.is_empty(), "{effects:?}");
            assert_eq!(outcome, Some(InspectorOutcome::FollowLink(target)));
        }
    }

    // Through the widget's reducer, as the app drives the child widget.

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn podman() -> EngineInfo {
        fixtures::engine(EngineKind::Podman)
    }

    fn lists(kind: EngineKind) -> LoadedLists {
        let resources = fixtures::resources(kind);

        LoadedLists {
            containers: Ok(resources.containers().to_vec()),
            images: Ok(resources.images().to_vec()),
            volumes: Ok(resources.volumes().to_vec()),
            networks: Ok(resources.networks().to_vec()),
        }
    }

    fn effects(task: Task<ContainersEvent>) -> Vec<ContainersEffect> {
        collect_outputs(task)
            .into_iter()
            .filter_map(|event| match event {
                ContainersEvent::Effect(effect) => Some(effect),
                ContainersEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Containers mode active, both fixture engines found, the Docker engine's
    /// lists loaded.
    fn loaded_state() -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        state
    }

    fn open(state: &mut ContainersState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce_containers(state, ContainersIntent::Open(place));
    }

    fn docker_containers() -> Vec<Container> {
        fixtures::resources(EngineKind::Docker)
            .containers()
            .to_vec()
    }

    // Inspector tabs.

    fn select_tab(
        state: &mut ContainersState,
        tab: InspectorTab,
    ) -> Vec<ContainersEffect> {
        effects(reduce_containers(
            state,
            ContainersIntent::SelectInspectorTab(tab),
        ))
    }

    fn container_id(index: usize) -> String {
        let resources = fixtures::resources(EngineKind::Docker);

        resources.containers()[index].id().to_string()
    }

    fn log_target(index: usize) -> LogTarget {
        LogTarget::new(docker(), &container_id(index))
    }

    fn followed(state: &ContainersState) -> Option<String> {
        let logs = state.inspector().logs()?;

        Some(logs.target().container_id().to_string())
    }

    fn log_line_count(state: &ContainersState) -> usize {
        state
            .inspector()
            .logs()
            .map_or(0, |logs| logs.lines().line_count())
    }

    fn push_lines(state: &mut ContainersState, index: usize, lines: &[&str]) {
        let lines = lines.iter().map(|line| line.to_string()).collect();
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: log_target(index),
                lines,
                position: None,
            }),
        );
    }

    fn inspect_of(resource: ResourceKind, id: &str) -> ContainersEffect {
        ContainersEffect::Inspector(InspectorEffect::LoadInspect {
            engine: docker(),
            target: InspectTarget::new(
                docker().address().clone(),
                resource,
                id,
            ),
        })
    }

    fn container_inspect(index: usize) -> InspectTarget {
        InspectTarget::new(
            docker().address().clone(),
            ResourceKind::Containers,
            &container_id(index),
        )
    }

    #[test]
    fn the_logs_follow_the_selected_container_while_the_logs_tab_is_open() {
        let mut state = loaded_state();
        let before = followed(&state);

        let effects = select_tab(&mut state, InspectorTab::Logs);

        assert_eq!(before, None);
        assert!(effects.is_empty());
        assert_eq!(state.inspector_tab(), InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(0)));
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.target(), &log_target(0));
        assert_eq!(logs.lines().line_count(), 0);
    }

    #[test]
    fn the_logs_stop_when_another_tab_opens_and_start_fresh_on_return() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["starting", "ready"]);
        let shown = log_line_count(&state);

        let _ = select_tab(&mut state, InspectorTab::Overview);
        let after_overview = followed(&state);
        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert_eq!(shown, 2);
        assert_eq!(after_overview, None);
        assert_eq!(followed(&state), Some(container_id(0)));
        assert_eq!(log_line_count(&state), 0);
    }

    #[test]
    fn the_logs_move_to_a_newly_selected_container_without_old_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["from the first container"]);

        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }

    #[test]
    fn the_logs_stop_when_containers_mode_is_left_and_follow_again_on_return() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        let _ = reduce_containers(&mut state, ContainersIntent::Leave);
        let after_leave = followed(&state);
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);

        assert_eq!(after_leave, None);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    fn hide_right_panel(
        state: &mut ContainersState,
        is_hidden: bool,
    ) -> Vec<ContainersEffect> {
        effects(reduce_containers(
            state,
            ContainersIntent::RightPanelHidden(is_hidden),
        ))
    }

    #[test]
    fn the_logs_stop_while_the_run_form_is_open_and_start_fresh_when_it_closes()
    {
        let mut state = logs_without_follow();

        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        let while_open = followed(&state);
        let tab_while_open = state.inspector_tab();
        push_lines(&mut state, 0, &["late line of the old stream"]);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Close),
        );

        assert_eq!(while_open, None);
        assert_eq!(tab_while_open, InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(0)));
        assert_eq!(log_line_count(&state), 0);
        assert!(log_view(&state).is_following());
        assert_eq!(followed_target(&state), log_target(0));
    }

    #[test]
    fn the_logs_stop_while_the_right_panel_is_hidden_and_start_fresh_when_shown()
     {
        let mut state = logs_without_follow();

        let _ = hide_right_panel(&mut state, true);
        let while_hidden = followed(&state);
        let tab_while_hidden = state.inspector_tab();
        push_lines(&mut state, 0, &["late line of the old stream"]);
        let _ = hide_right_panel(&mut state, false);

        assert_eq!(while_hidden, None);
        assert_eq!(tab_while_hidden, InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(0)));
        assert_eq!(log_line_count(&state), 0);
        assert!(log_view(&state).is_following());
        assert_eq!(followed_target(&state), log_target(0));
    }

    #[test]
    fn selecting_the_logs_tab_while_the_right_panel_is_hidden_starts_no_logs() {
        let mut state = loaded_state();
        let _ = hide_right_panel(&mut state, true);

        let _ = select_tab(&mut state, InspectorTab::Logs);
        let while_hidden = followed(&state);
        let tab_while_hidden = state.inspector_tab();
        let _ = hide_right_panel(&mut state, false);

        assert_eq!(while_hidden, None);
        assert_eq!(tab_while_hidden, InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    #[test]
    fn logs_need_the_panel_shown_and_the_form_closed_at_once() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let _ = hide_right_panel(&mut state, true);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let _ = hide_right_panel(&mut state, false);
        let form_still_open = followed(&state);
        let _ = hide_right_panel(&mut state, true);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Close),
        );
        let panel_still_hidden = followed(&state);
        let _ = hide_right_panel(&mut state, false);

        assert_eq!(form_still_open, None);
        assert_eq!(panel_still_hidden, None);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    #[test]
    fn a_new_mode_does_not_bring_the_logs_back_while_the_right_panel_is_hidden()
    {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let _ = reduce_containers(&mut state, ContainersIntent::Leave);
        let _ = hide_right_panel(&mut state, true);

        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let after_enter = followed(&state);
        let _ = hide_right_panel(&mut state, false);

        assert_eq!(after_enter, None);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    #[test]
    fn the_inspect_json_waits_for_the_right_panel_and_loads_once() {
        let mut state = loaded_state();
        let _ = hide_right_panel(&mut state, true);

        let while_hidden = select_tab(&mut state, InspectorTab::Inspect);
        let on_show = hide_right_panel(&mut state, false);
        let _ = hide_right_panel(&mut state, true);
        let on_second_show = hide_right_panel(&mut state, false);

        assert!(while_hidden.is_empty());
        assert_eq!(
            on_show,
            [inspect_of(ResourceKind::Containers, &container_id(0))]
        );
        assert!(on_second_show.is_empty());
    }

    #[test]
    fn the_inspect_json_waits_for_the_run_form_to_close() {
        let mut state = loaded_state();
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let while_open = select_tab(&mut state, InspectorTab::Inspect);
        let on_close = effects(reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Close),
        ));

        assert!(while_open.is_empty());
        assert_eq!(
            on_close,
            [inspect_of(ResourceKind::Containers, &container_id(0))]
        );
    }

    #[test]
    fn pressing_the_selected_row_keeps_its_logs_and_their_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one"]);
        let stream = followed_target(&state);

        let position = state.shown_position(0);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_line_count(&state), 1);
    }

    #[test]
    fn a_reload_that_keeps_the_container_keeps_its_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one", "two"]);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );

        assert_eq!(followed(&state), Some(container_id(0)));
        assert_eq!(log_line_count(&state), 2);
    }

    #[test]
    fn a_container_that_is_gone_is_no_longer_followed() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let mut empty = lists(EngineKind::Docker);
        empty.containers = Ok(Vec::new());

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: empty,
            },
        );

        assert_eq!(followed(&state), None);
    }

    #[test]
    fn only_containers_have_a_logs_tab() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert_eq!(state.inspector_tab(), InspectorTab::Overview);
        assert_eq!(followed(&state), None);
    }

    #[test]
    fn opening_another_list_goes_back_to_the_overview() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        open(&mut state, ResourceKind::Images);
        let on_images = state.inspector_tab();
        open(&mut state, ResourceKind::Containers);

        assert_eq!(on_images, InspectorTab::Overview);
        assert_eq!(state.inspector_tab(), InspectorTab::Overview);
        assert_eq!(followed(&state), None);
    }

    /// The user opened no list: the view moved on to the containers of the
    /// engine that is left, so the open tab stays.
    #[test]
    fn a_place_moved_from_a_lost_engine_keeps_the_open_tab() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Podman,
            ])),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );

        assert_eq!(
            state.place(),
            Some(&Place::new(
                podman().address().clone(),
                ResourceKind::Containers
            ))
        );
        assert_eq!(state.inspector_tab(), InspectorTab::Logs);
        assert_eq!(followed_target(&state).engine(), &podman());
    }

    #[test]
    fn at_most_5000_log_lines_are_kept() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let lines: Vec<String> = (0..MAX_LOG_LINES + 100)
            .map(|n| format!("line {n}"))
            .collect();
        let (first, second) = lines.split_at(4_000);

        for batch in [first, second] {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                }),
            );
        }

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(MAX_LOG_LINES, 5000);
        assert!(logs.lines().line_count() <= MAX_LOG_LINES);
        assert_eq!(logs.lines().line_count(), 4000);
        let text = logs.lines().text();
        assert!(text.starts_with("line 1100\n"), "{}", &text[..20]);
        assert!(text.ends_with("line 5099"));
    }

    fn log_view(state: &ContainersState) -> &LogViewState {
        state.inspector().logs().expect("followed logs").lines()
    }

    fn set_follow(state: &mut ContainersState, is_on: bool) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogFollowToggled(
                is_on,
            )),
        );
    }

    /// The Logs tab of the first container with two lines, Follow off.
    fn logs_without_follow() -> ContainersState {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one", "two"]);
        set_follow(&mut state, false);
        state
    }

    #[test]
    fn follow_is_on_when_the_logs_open() {
        let mut state = loaded_state();

        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert!(log_view(&state).is_following());
    }

    #[test]
    fn with_follow_off_new_lines_wait_and_the_view_keeps_its_lines() {
        let mut state = logs_without_follow();

        push_lines(&mut state, 0, &["three", "four", "five"]);

        let lines = log_view(&state);
        assert!(!lines.is_following());
        assert_eq!(lines.text(), "one\ntwo");
        assert_eq!(lines.pending_count(), 3);
    }

    #[test]
    fn turning_follow_on_adds_the_waiting_lines() {
        let mut state = logs_without_follow();
        push_lines(&mut state, 0, &["three"]);

        set_follow(&mut state, true);

        let lines = log_view(&state);
        assert!(lines.is_following());
        assert_eq!(lines.text(), "one\ntwo\nthree");
        assert_eq!(lines.pending_count(), 0);
    }

    #[test]
    fn scrolling_up_in_the_logs_turns_follow_off() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one", "two"]);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogAction(
                Action::Scroll { lines: -2 },
            )),
        );
        push_lines(&mut state, 0, &["three"]);

        assert!(!log_view(&state).is_following());
        assert_eq!(log_view(&state).pending_count(), 1);
    }

    #[test]
    fn the_5000_line_cap_holds_while_follow_is_off() {
        let mut state = logs_without_follow();
        let lines: Vec<String> = (0..MAX_LOG_LINES + 100)
            .map(|n| format!("line {n}"))
            .collect();

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: log_target(0),
                lines,
                position: None,
            }),
        );
        let waiting = log_view(&state).pending_count();
        set_follow(&mut state, true);

        assert_eq!(waiting, MAX_LOG_LINES);
        let lines = log_view(&state);
        assert!(lines.line_count() <= MAX_LOG_LINES);
        assert_eq!(lines.line_count(), 4000);
        let text = lines.text();
        assert!(text.starts_with("line 1100\n"), "{}", &text[..20]);
        assert!(text.ends_with("line 5099"));
    }

    /// Lines of the longest kind, enough for more than the whole byte limit,
    /// each starting with its number.
    fn longest_log_lines() -> Vec<String> {
        let count = MAX_LOG_TEXT_BYTES / MAX_LOG_LINE_BYTES + 20;

        (0..count)
            .map(|n| format!("{n:05}{}", "x".repeat(MAX_LOG_LINE_BYTES - 5)))
            .collect()
    }

    fn push_batches(state: &mut ContainersState, lines: &[String]) {
        for batch in lines.chunks(50) {
            let _ = reduce_containers(
                state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                }),
            );
        }
    }

    /// The bytes the logs count against their limit, shown and waiting lines
    /// together, where `pushed` are the lines pushed last, which the waiting
    /// ones are the newest of.
    fn kept_log_bytes(state: &ContainersState, pushed: &[String]) -> usize {
        let lines = log_view(state);
        let shown = lines.text().len() + 1;
        let waiting: usize = pushed
            .iter()
            .rev()
            .take(lines.pending_count())
            .map(|line| line.len() + 1)
            .sum();

        shown + waiting
    }

    #[test]
    fn at_most_16_mib_of_log_text_is_kept_and_the_newest_stays() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let lines = longest_log_lines();

        push_batches(&mut state, &lines);

        assert_eq!(MAX_LOG_TEXT_BYTES, 16 * 1024 * 1024);
        let text = log_view(&state).text();
        assert!(text.len() <= MAX_LOG_TEXT_BYTES, "{}", text.len());
        assert!(
            text.len() > MAX_LOG_TEXT_BYTES / 5 * 4 - 2 * MAX_LOG_LINE_BYTES
        );
        assert!(!text.starts_with("00000"), "the oldest go first");
        let newest = lines.last().expect("lines");
        assert!(text.ends_with(newest.as_str()));
    }

    /// Pushes `lines` in batches of `batch_size`, checks both limits and the
    /// newest line after each batch, and returns how many batches shrank the
    /// log.
    fn push_and_count_trims(
        state: &mut ContainersState,
        lines: &[String],
        batch_size: usize,
    ) -> usize {
        let mut trims = 0;

        for batch in lines.chunks(batch_size) {
            let before = log_view(state).line_count();

            push_batches(state, batch);

            let text = log_view(state).text();
            let newest = batch.last().expect("batch");
            assert!(log_view(state).line_count() <= MAX_LOG_LINES);
            assert!(text.len() <= MAX_LOG_TEXT_BYTES, "{}", text.len());
            assert!(text.ends_with(newest.as_str()));
            if log_view(state).line_count() < before {
                trims += 1;
            }
        }

        trims
    }

    #[test]
    fn a_full_log_of_short_lines_keeps_its_cap_and_trims_rarely() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let lines: Vec<String> =
            (0..15_000).map(|n| format!("line {n}")).collect();

        let trims = push_and_count_trims(&mut state, &lines, 50);

        // Of 300 batches of 50: the first trim comes at 5050 lines and leaves
        // 4000, so the next one comes 21 batches later.
        assert_eq!(trims, 10);
    }

    #[test]
    fn a_full_log_of_long_lines_keeps_its_cap_and_trims_rarely() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let lines = [longest_log_lines(), longest_log_lines()].concat();

        let trims = push_and_count_trims(&mut state, &lines, 25);

        // Of 23 batches of 25: 255 of these lines fit, the first trim comes at
        // 275 and leaves 204, so the next one comes 3 batches later.
        assert_eq!(trims, 5);
    }

    #[test]
    fn the_16_mib_cap_holds_for_the_lines_that_wait_while_follow_is_off() {
        let mut state = logs_without_follow();
        let lines = longest_log_lines();

        push_batches(&mut state, &lines);

        let waiting = log_view(&state).pending_count();
        assert!(waiting < lines.len());
        assert_eq!(log_view(&state).text(), "one\ntwo");
        let kept = kept_log_bytes(&state, &lines);
        assert!(kept <= MAX_LOG_TEXT_BYTES, "{kept}");
        assert!(kept > MAX_LOG_TEXT_BYTES - 2 * MAX_LOG_LINE_BYTES, "{kept}");

        set_follow(&mut state, true);

        let text = log_view(&state).text();
        assert!(text.len() <= MAX_LOG_TEXT_BYTES, "{}", text.len());
        let newest = lines.last().expect("lines");
        assert!(text.ends_with(newest.as_str()));
    }

    /// Nearly 16 MiB shown and the user scrolls up, so Follow is off.
    fn full_log_scrolled_up(state: &mut ContainersState) {
        push_batches(state, &longest_log_lines()[..255]);

        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogAction(
                Action::Scroll { lines: -2 },
            )),
        );
    }

    /// Lines of the longest kind that can be told from those of
    /// [`longest_log_lines`], enough to go over the whole byte limit.
    fn arriving_log_lines() -> Vec<String> {
        longest_log_lines()
            .into_iter()
            .map(|line| format!("new{line}")[..MAX_LOG_LINE_BYTES].to_string())
            .collect()
    }

    /// The input of the 32 MiB bug: 16 MiB shown, the user scrolls up, 16 MiB
    /// more arrives. Only the newest line can wait, the text being read stays,
    /// and Follow brings back the old text, then the line that says output was
    /// skipped, then the newest line.
    #[test]
    fn a_full_log_with_follow_off_keeps_its_text_and_only_the_newest_line_waits()
     {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        full_log_scrolled_up(&mut state);
        let shown = log_view(&state).text();
        assert!(shown.len() > MAX_LOG_TEXT_BYTES - MAX_LOG_LINE_BYTES);
        let arriving = arriving_log_lines();

        for batch in arriving.chunks(50) {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                }),
            );
            assert!(
                kept_log_bytes(&state, &arriving)
                    <= MAX_LOG_TEXT_BYTES + MAX_LOG_LINE_BYTES + 1
            );
        }

        assert_eq!(log_view(&state).pending_count(), 1);
        assert_eq!(log_view(&state).text(), shown);

        set_follow(&mut state, true);

        let text = log_view(&state).text();
        let newest = arriving.last().expect("lines");
        let skipped = i18n::t(Key::ContainersLogsSkippedOutput);
        assert!(text.ends_with(&format!("\n{skipped}\n{newest}")));
        assert_eq!(text.matches(skipped).count(), 1);
        assert_eq!(text.matches("new").count(), 1);
        assert!(
            text.len() <= MAX_LOG_TEXT_BYTES / 5 * 4 + 1,
            "{}",
            text.len()
        );
        assert_eq!(log_view(&state).pending_count(), 0);
    }

    #[test]
    fn dropped_waiting_lines_are_forgotten_when_the_container_restarts() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        full_log_scrolled_up(&mut state);
        push_batches(&mut state, &arriving_log_lines());
        assert_eq!(log_view(&state).pending_count(), 1);

        container_started(&mut state, 0);
        lines_arrived(&mut state, &["after the restart"], None);
        set_follow(&mut state, true);

        assert_eq!(log_view(&state).text(), "after the restart");
    }

    #[test]
    fn no_lines_dropped_means_no_skipped_output_line() {
        let mut state = logs_without_follow();

        push_lines(&mut state, 0, &["three"]);
        set_follow(&mut state, true);

        let skipped = i18n::t(Key::ContainersLogsSkippedOutput);
        assert!(!log_view(&state).text().contains(skipped));
    }

    #[test]
    fn logs_that_start_over_keep_the_byte_limit() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::ContainerStarted {
                engine: docker().address().clone(),
                id: container_id(0),
            }),
        );
        let target = state
            .inspector()
            .logs()
            .expect("followed logs")
            .target()
            .clone();

        for batch in longest_log_lines().chunks(50) {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target: target.clone(),
                    lines: batch.to_vec(),
                    position: None,
                }),
            );
        }

        let text = log_view(&state).text();
        assert!(text.len() <= MAX_LOG_TEXT_BYTES, "{}", text.len());
    }

    #[test]
    fn follow_is_on_again_after_a_tab_change() {
        let mut state = logs_without_follow();

        let _ = select_tab(&mut state, InspectorTab::Overview);
        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert!(log_view(&state).is_following());
    }

    #[test]
    fn follow_is_on_again_after_another_container_is_selected() {
        let mut state = logs_without_follow();

        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );

        assert_eq!(followed(&state), Some(container_id(1)));
        assert!(log_view(&state).is_following());
    }

    #[test]
    fn follow_is_on_again_after_a_mode_change() {
        let mut state = logs_without_follow();

        let _ = reduce_containers(&mut state, ContainersIntent::Leave);
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);

        assert!(log_view(&state).is_following());
    }

    #[test]
    fn a_follow_toggle_without_followed_logs_changes_nothing() {
        let mut state = loaded_state();

        set_follow(&mut state, false);

        assert_eq!(followed(&state), None);
    }

    #[test]
    fn lines_of_a_container_no_longer_followed_are_dropped() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        push_lines(&mut state, 1, &["from another container"]);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: log_target(1),
                error: ContainersError::new("gone"),
            }),
        );

        assert_eq!(log_line_count(&state), 0);
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    /// A start event of the container at `index` of the fixture Docker engine.
    fn container_started(state: &mut ContainersState, index: usize) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::ContainerStarted {
                engine: docker().address().clone(),
                id: container_id(index),
            }),
        );
    }

    /// The target of the followed logs.
    fn followed_target(state: &ContainersState) -> LogTarget {
        state
            .inspector()
            .logs()
            .expect("followed logs")
            .target()
            .clone()
    }

    #[test]
    fn a_started_container_clears_its_logs_and_asks_for_a_new_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["old one", "old two"]);
        let old_stream = followed_target(&state);

        container_started(&mut state, 0);

        assert_eq!(log_view(&state).text(), "");
        let new_stream = followed_target(&state);
        assert_ne!(new_stream, old_stream);
        assert!(new_stream.is_same_container(&old_stream));
        assert_eq!(new_stream.read(), LogRead::LatestRun);
        assert_eq!(old_stream.read(), LogRead::LastLines);
    }

    #[test]
    fn each_start_is_one_new_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        container_started(&mut state, 0);
        let first = followed_target(&state);
        container_started(&mut state, 0);
        let second = followed_target(&state);

        assert_ne!(first, second);
    }

    /// The old stream can still deliver lines after the start event: Podman's
    /// keeps going across a restart. They must not show twice.
    #[test]
    fn late_lines_of_the_old_stream_are_dropped_and_new_lines_show_once() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["old"]);
        let old_stream = followed_target(&state);

        container_started(&mut state, 0);
        let new_stream = followed_target(&state);
        for (target, line) in [(old_stream, "fresh"), (new_stream, "fresh")] {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target,
                    lines: vec![line.to_string()],
                    position: None,
                }),
            );
        }

        assert_eq!(log_view(&state).text(), "fresh");
    }

    #[test]
    fn a_late_error_of_the_old_stream_is_not_shown() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let old_stream = followed_target(&state);

        container_started(&mut state, 0);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: old_stream,
                error: ContainersError::new("stream broke"),
            }),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    #[test]
    fn a_started_container_forgets_the_error_of_its_old_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: log_target(0),
                error: ContainersError::new("stream broke"),
            }),
        );

        container_started(&mut state, 0);

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    #[test]
    fn a_start_keeps_follow_off_and_forgets_the_waiting_lines() {
        let mut state = logs_without_follow();
        push_lines(&mut state, 0, &["waiting"]);

        container_started(&mut state, 0);

        let view = log_view(&state);
        assert!(!view.is_following());
        assert_eq!(view.pending_count(), 0);
        assert_eq!(view.text(), "");

        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: stream,
                lines: vec![String::from("fresh")],
                position: None,
            }),
        );
        assert_eq!(log_view(&state).pending_count(), 1);
        set_follow(&mut state, true);
        assert_eq!(log_view(&state).text(), "fresh");
    }

    #[test]
    fn a_start_keeps_follow_on_and_the_line_cap() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        container_started(&mut state, 0);
        let stream = followed_target(&state);
        let lines: Vec<String> = (0..MAX_LOG_LINES + 1)
            .map(|n| format!("line {n}"))
            .collect();
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: stream,
                lines,
                position: None,
            }),
        );

        assert!(log_view(&state).is_following());
        assert_eq!(log_view(&state).line_count(), MAX_LOG_LINES);
    }

    /// The list reload that follows the start event must not undo it.
    #[test]
    fn a_list_reload_after_a_start_keeps_the_new_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        container_started(&mut state, 0);
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: stream.clone(),
                lines: vec![String::from("fresh")],
                position: None,
            }),
        );

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(
                    lists(EngineKind::Docker).containers,
                ),
            },
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "fresh");
    }

    #[test]
    fn the_start_of_another_container_changes_nothing() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["kept"]);
        let stream = followed_target(&state);

        container_started(&mut state, 1);

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "kept");
    }

    #[test]
    fn the_start_of_a_container_with_the_same_id_elsewhere_changes_nothing() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["kept"]);
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::ContainerStarted {
                engine: podman().address().clone(),
                id: container_id(0),
            }),
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "kept");
    }

    #[test]
    fn a_start_without_followed_logs_starts_no_logs() {
        let mut state = loaded_state();

        container_started(&mut state, 0);

        assert_eq!(followed(&state), None);
    }

    /// Without a start event nothing asks for a new stream, so a container
    /// that stays stopped keeps its lines and the engine is left alone.
    #[test]
    fn a_stopped_container_keeps_its_lines_and_its_stream_key() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["last words"]);
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListChanged {
                engine: docker().address().clone(),
                resource: ResourceKind::Containers,
            },
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(
                    lists(EngineKind::Docker).containers,
                ),
            },
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "last words");
    }

    #[test]
    fn logs_opened_again_after_a_start_begin_with_the_last_lines_again() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        container_started(&mut state, 0);

        let _ = select_tab(&mut state, InspectorTab::Overview);
        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
    }

    #[test]
    fn another_selection_after_a_start_begins_with_the_last_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        container_started(&mut state, 0);

        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
    }

    #[test]
    fn a_failed_log_stream_shows_the_engine_message_and_keeps_the_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["starting"]);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: log_target(0),
                error: ContainersError::new("No such container"),
            }),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("No such container"));
        assert_eq!(logs.lines().text(), "starting");
    }

    #[test]
    fn inspect_loads_once_per_selection_when_its_tab_opens() {
        let mut state = loaded_state();

        let first = select_tab(&mut state, InspectorTab::Inspect);
        let overview = select_tab(&mut state, InspectorTab::Overview);
        let again = select_tab(&mut state, InspectorTab::Inspect);

        assert_eq!(
            first,
            vec![inspect_of(ResourceKind::Containers, &container_id(0))]
        );
        assert!(overview.is_empty());
        assert!(again.is_empty());
        let inspect = state.inspector().inspect().expect("inspect asked for");
        assert_eq!(inspect.target(), &container_inspect(0));
        assert!(matches!(inspect.load(), InspectLoad::Loading));
    }

    #[test]
    fn a_new_selection_loads_its_inspect_while_the_tab_is_open() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        let position = state.shown_position(1);
        let other = effects(reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        ));
        let same = effects(reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        ));

        assert!(
            other.contains(&inspect_of(
                ResourceKind::Containers,
                &container_id(1)
            ))
        );
        assert_eq!(same, vec![ContainersEffect::RevealInspector]);
    }

    #[test]
    fn a_selection_on_the_overview_loads_when_inspect_opens() {
        let mut state = loaded_state();

        let position = state.shown_position(1);
        let on_overview = effects(reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        ));
        let opened = select_tab(&mut state, InspectorTab::Inspect);

        assert_eq!(on_overview, vec![ContainersEffect::RevealInspector]);
        assert_eq!(
            opened,
            vec![inspect_of(ResourceKind::Containers, &container_id(1))]
        );
    }

    #[test]
    fn each_list_inspects_its_selected_row() {
        let mut state = loaded_state();
        let resources = fixtures::resources(EngineKind::Docker);
        let mut asked = Vec::new();

        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ] {
            open(&mut state, resource);
            asked.extend(select_tab(&mut state, InspectorTab::Inspect));
        }

        assert_eq!(
            asked,
            vec![
                inspect_of(ResourceKind::Images, resources.images()[0].id()),
                inspect_of(
                    ResourceKind::Volumes,
                    resources.volumes()[0].name()
                ),
                inspect_of(
                    ResourceKind::Networks,
                    resources.networks()[0].id()
                ),
            ]
        );
    }

    #[test]
    fn the_inspect_json_is_kept_and_an_answer_for_another_row_is_dropped() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);
        let json = String::from("{\n  \"Id\": \"850b\"\n}");

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::InspectLoaded {
                target: container_inspect(0),
                result: Ok(json.clone()),
            }),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::InspectLoaded {
                target: container_inspect(1),
                result: Err(ContainersError::new("late")),
            }),
        );

        let inspect = state.inspector().inspect().expect("inspect");
        assert_eq!(inspect.target(), &container_inspect(0));
        let InspectLoad::Loaded(shown) = inspect.load() else {
            panic!("expected the json, got {:?}", inspect.load());
        };
        assert_eq!(shown.json().text(), json);
    }

    #[test]
    fn a_failed_inspect_shows_the_engine_message() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::InspectLoaded {
                target: container_inspect(0),
                result: Err(ContainersError::new("No such container")),
            }),
        );

        let inspect = state.inspector().inspect().expect("inspect");
        assert!(matches!(
            inspect.load(),
            InspectLoad::Failed(message) if message == "No such container"
        ));
    }

    #[test]
    fn nothing_is_inspected_outside_containers_mode() {
        let mut state = loaded_state();
        let _ = reduce_containers(&mut state, ContainersIntent::Leave);

        let effects = select_tab(&mut state, InspectorTab::Inspect);

        assert!(effects.is_empty());
        assert!(state.inspector().inspect().is_none());
    }

    // Inspect as a tree (ticket 32).

    /// A State, a Config and a Mounts section.
    const TREE_JSON: &str = r#"{
        "Id": "850b",
        "Mounts": [{ "Type": "bind" }],
        "Config": { "Image": "busybox" },
        "State": { "Status": "running" }
    }"#;

    /// Containers mode on the Inspect tab of the first container, with `json`
    /// loaded.
    fn state_with_inspect(json: &str) -> ContainersState {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);
        load_inspect(&mut state, 0, json);
        state
    }

    fn load_inspect(state: &mut ContainersState, index: usize, json: &str) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::InspectLoaded {
                target: container_inspect(index),
                result: Ok(json.to_string()),
            }),
        );
    }

    fn toggle_section(state: &mut ContainersState, title: &str) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(
                InspectorIntent::InspectSectionToggled(title.to_string()),
            ),
        );
    }

    fn open_sections(state: &ContainersState) -> Vec<String> {
        let inspect = state.inspector().inspect().expect("inspect");
        let InspectLoad::Loaded(loaded) = inspect.load() else {
            panic!("expected the json, got {:?}", inspect.load());
        };

        loaded.open_sections().iter().cloned().collect()
    }

    #[test]
    fn only_the_first_section_is_open_when_the_json_arrives() {
        let state = state_with_inspect(TREE_JSON);

        assert_eq!(open_sections(&state), vec!["State"]);
    }

    #[test]
    fn the_first_section_shown_is_the_open_one() {
        let state = state_with_inspect(
            r#"{ "State": { "Error": "" }, "Config": { "Image": "busybox" } }"#,
        );

        assert_eq!(open_sections(&state), vec!["Config"]);
    }

    #[test]
    fn pressing_a_section_title_opens_and_closes_it() {
        let mut state = state_with_inspect(TREE_JSON);

        toggle_section(&mut state, "Config");
        let both = open_sections(&state);
        toggle_section(&mut state, "State");
        let config = open_sections(&state);
        toggle_section(&mut state, "Config");
        let none = open_sections(&state);

        assert_eq!(both, vec!["Config", "State"]);
        assert_eq!(config, vec!["Config"]);
        assert!(none.is_empty());
    }

    #[test]
    fn pressing_a_section_title_before_the_json_arrives_changes_nothing() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        toggle_section(&mut state, "State");

        let inspect = state.inspector().inspect().expect("inspect");
        assert!(matches!(inspect.load(), InspectLoad::Loading));
    }

    #[test]
    fn the_open_sections_are_kept_over_a_tab_change() {
        let mut state = state_with_inspect(TREE_JSON);
        toggle_section(&mut state, "Mounts");

        let _ = select_tab(&mut state, InspectorTab::Overview);
        let back = select_tab(&mut state, InspectorTab::Inspect);

        assert!(back.is_empty());
        assert_eq!(open_sections(&state), vec!["Mounts", "State"]);
    }

    #[test]
    fn another_selection_starts_with_the_default_sections() {
        let mut state = state_with_inspect(TREE_JSON);
        toggle_section(&mut state, "Config");
        toggle_section(&mut state, "State");

        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );
        load_inspect(&mut state, 1, TREE_JSON);

        assert_eq!(open_sections(&state), vec!["State"]);
    }

    #[test]
    fn a_selection_changed_on_another_tab_also_forgets_the_open_sections() {
        let mut state = state_with_inspect(TREE_JSON);
        toggle_section(&mut state, "Config");
        let _ = select_tab(&mut state, InspectorTab::Overview);

        let other = state.shown_position(1);
        let _ =
            reduce_containers(&mut state, ContainersIntent::SelectRow(other));
        let first = state.shown_position(0);
        let _ =
            reduce_containers(&mut state, ContainersIntent::SelectRow(first));
        let opened = select_tab(&mut state, InspectorTab::Inspect);
        load_inspect(&mut state, 0, TREE_JSON);

        assert_eq!(
            opened,
            vec![inspect_of(ResourceKind::Containers, &container_id(0))]
        );
        assert_eq!(open_sections(&state), vec!["State"]);
    }

    #[test]
    fn the_raw_json_and_show_empty_switches_stay_over_a_new_selection() {
        let mut state = state_with_inspect(TREE_JSON);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(
                InspectorIntent::InspectRawJsonToggled(true),
            ),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(
                InspectorIntent::InspectShowEmptyToggled(true),
            ),
        );
        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );

        assert!(state.inspector().is_raw_json());
        assert!(state.inspector().shows_empty());

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(
                InspectorIntent::InspectRawJsonToggled(false),
            ),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(
                InspectorIntent::InspectShowEmptyToggled(false),
            ),
        );

        assert!(!state.inspector().is_raw_json());
        assert!(!state.inspector().shows_empty());
    }

    #[test]
    fn copy_json_asks_for_the_clipboard_with_the_json_text() {
        let mut state = state_with_inspect("{\n  \"Id\": \"850b\"\n}");

        let effects = effects(reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::CopyInspectJson),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::Inspector(
                InspectorEffect::CopyToClipboard(String::from(
                    "{\n  \"Id\": \"850b\"\n}"
                ))
            )]
        );
    }

    #[test]
    fn copy_json_does_nothing_before_the_json_arrives() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        let effects = effects(reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::CopyInspectJson),
        ));

        assert!(effects.is_empty());
    }

    #[test]
    fn an_answer_that_is_not_an_object_keeps_its_text_and_opens_no_section() {
        let mut state = state_with_inspect("not json");

        let copied = effects(reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::CopyInspectJson),
        ));

        assert!(open_sections(&state).is_empty());
        assert_eq!(
            copied,
            vec![ContainersEffect::Inspector(
                InspectorEffect::CopyToClipboard(String::from("not json"))
            )]
        );
    }

    #[test]
    fn an_event_reload_keeps_the_logs_of_a_kept_container_and_stops_a_gone_one()
    {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["ready"]);
        let reload =
            |containers: Vec<Container>| ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Ok(containers)),
            };

        let _ = reduce_containers(&mut state, reload(docker_containers()));
        let kept = (followed(&state), log_line_count(&state));
        let _ = reduce_containers(
            &mut state,
            reload(docker_containers()[1..].to_vec()),
        );

        assert_eq!(kept, (Some(container_id(0)), 1));
        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }

    // Logs after a reconnect (spec Decision 12).

    const RUN_STARTED: i64 = 1_793_750_700;

    const RUN_STARTED_AGAIN: i64 = 1_793_750_800;

    /// The first container on the Logs tab, its stream having read a running
    /// run and two lines, the last one at the position `seen()`.
    fn open_logs_with_two_lines() -> ContainersState {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        lines_arrived(&mut state, &["one", "two"], Some(seen()));
        state
    }

    fn seen() -> LogPosition {
        fixtures::log_position(RUN_STARTED + 5, 2)
    }

    /// The followed stream read the run started at `started`.
    fn run_read(state: &mut ContainersState, started: i64) {
        let target = followed_target(state);
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogRunRead {
                target,
                run: fixtures::container_run(started, true),
            }),
        );
    }

    /// The followed stream sent `lines`, ending at `position`.
    fn lines_arrived(
        state: &mut ContainersState,
        lines: &[&str],
        position: Option<LogPosition>,
    ) {
        let target = followed_target(state);
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target,
                lines: lines.iter().map(|line| line.to_string()).collect(),
                position,
            }),
        );
    }

    /// The answer about the followed stream's container came.
    fn run_checked(
        state: &mut ContainersState,
        result: Result<ContainerRun, ContainersError>,
    ) {
        let target = followed_target(state);
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                target,
                result,
            }),
        );
    }

    fn reconnected(engine: &EngineInfo) -> ContainersIntent {
        ContainersIntent::EngineReconnected(engine.address().clone())
    }

    #[test]
    fn a_reconnect_asks_about_the_open_logs_and_reloads_the_lists() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let effects =
            effects(reduce_containers(&mut state, reconnected(&docker())));

        assert_eq!(
            effects,
            vec![
                ContainersEffect::LoadLists(docker()),
                ContainersEffect::Inspector(InspectorEffect::CheckLogRun(
                    stream.clone()
                )),
            ]
        );
        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    #[test]
    fn a_reconnect_of_another_engine_leaves_the_open_logs_alone() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let effects =
            effects(reduce_containers(&mut state, reconnected(&podman())));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(podman())]);
        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    #[test]
    fn a_reconnect_with_nothing_open_only_reloads_the_lists() {
        let mut state = loaded_state();

        let effects =
            effects(reduce_containers(&mut state, reconnected(&docker())));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
        assert_eq!(followed(&state), None);
    }

    #[test]
    fn a_reconnect_of_the_engine_of_another_tab_starts_no_logs() {
        let mut state = open_logs_with_two_lines();
        let _ = select_tab(&mut state, InspectorTab::Overview);

        let effects =
            effects(reduce_containers(&mut state, reconnected(&docker())));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
        assert_eq!(followed(&state), None);
    }

    #[test]
    fn a_restart_in_the_gap_clears_the_logs_and_reads_only_the_new_run() {
        let mut state = open_logs_with_two_lines();
        let old_stream = followed_target(&state);

        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
        );

        assert_eq!(log_view(&state).text(), "");
        let new_stream = followed_target(&state);
        assert_ne!(new_stream, old_stream);
        assert_eq!(new_stream.read(), LogRead::LatestRun);
    }

    #[test]
    fn the_same_run_keeps_the_lines_and_reads_on_after_them() {
        let mut state = open_logs_with_two_lines();
        let old_stream = followed_target(&state);

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        assert_eq!(log_view(&state).text(), "one\ntwo");
        let new_stream = followed_target(&state);
        assert_ne!(new_stream, old_stream);
        assert!(new_stream.is_same_container(&old_stream));
        assert_eq!(new_stream.read(), LogRead::After(seen()));

        lines_arrived(&mut state, &["three"], None);

        assert_eq!(log_view(&state).text(), "one\ntwo\nthree");
    }

    /// Only the events stream broke: the old log stream is alive and can still
    /// deliver. What it sends after the switch comes through the new stream,
    /// from the position the view had at the switch, so it shows once.
    #[test]
    fn a_log_stream_that_survived_repeats_nothing() {
        let mut state = open_logs_with_two_lines();
        let old_stream = followed_target(&state);

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogLines {
                target: old_stream.clone(),
                lines: vec![String::from("three")],
                position: Some(fixtures::log_position(RUN_STARTED + 6, 1)),
            }),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: old_stream,
                error: ContainersError::new("old stream dropped"),
            }),
        );

        assert_eq!(log_view(&state).text(), "one\ntwo");
        assert_eq!(followed_target(&state).read(), LogRead::After(seen()));
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);

        lines_arrived(&mut state, &["three"], None);

        assert_eq!(log_view(&state).text(), "one\ntwo\nthree");
    }

    #[test]
    fn the_stream_goes_on_from_the_last_line_that_arrived() {
        let mut state = open_logs_with_two_lines();
        let last = fixtures::log_position(RUN_STARTED + 6, 1);
        lines_arrived(&mut state, &["three"], Some(last));
        lines_arrived(&mut state, &["piece without a time"], None);

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        assert_eq!(followed_target(&state).read(), LogRead::After(last));
    }

    /// The second stream broke before it sent a line: the same position, yet
    /// it must be another stream, or iced keeps the dead one.
    #[test]
    fn a_second_reconnect_from_the_same_position_is_another_stream() {
        let mut state = open_logs_with_two_lines();
        let run = fixtures::container_run(RUN_STARTED, true);

        run_checked(&mut state, Ok(run));
        let second = followed_target(&state);
        run_checked(&mut state, Ok(run));
        let third = followed_target(&state);

        assert_ne!(second, third);
        assert_eq!(second.read(), third.read());
    }

    #[test]
    fn going_on_forgets_the_error_of_the_broken_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            }),
        );

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
        assert_eq!(logs.lines().text(), "one\ntwo");
    }

    #[test]
    fn with_follow_off_new_lines_keep_waiting_across_a_reconnect() {
        let mut state = open_logs_with_two_lines();
        set_follow(&mut state, false);
        lines_arrived(
            &mut state,
            &["waiting"],
            Some(fixtures::log_position(RUN_STARTED + 6, 1)),
        );

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        let stream = followed_target(&state);
        assert_eq!(
            stream.read(),
            LogRead::After(fixtures::log_position(RUN_STARTED + 6, 1)),
            "the waiting line must not come again"
        );
        lines_arrived(&mut state, &["missed"], None);
        let view = log_view(&state);
        assert!(!view.is_following());
        assert_eq!(view.pending_count(), 2);
        assert_eq!(view.text(), "one\ntwo");

        set_follow(&mut state, true);

        assert_eq!(log_view(&state).text(), "one\ntwo\nwaiting\nmissed");
    }

    /// The line that says output was skipped exists only in the view: the new
    /// stream still goes on from the last line that arrived.
    #[test]
    fn the_skipped_output_line_leaves_the_log_position_alone() {
        let mut state = open_logs_with_two_lines();
        full_log_scrolled_up(&mut state);
        let last = fixtures::log_position(RUN_STARTED + 6, 1);
        let arriving = arriving_log_lines();

        for batch in arriving.chunks(50) {
            let target = followed_target(&state);
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogLines {
                    target,
                    lines: batch.to_vec(),
                    position: Some(last),
                }),
            );
        }
        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));
        assert_eq!(followed_target(&state).read(), LogRead::After(last));
        set_follow(&mut state, true);

        let text = log_view(&state).text();
        let skipped = i18n::t(Key::ContainersLogsSkippedOutput);
        assert_eq!(text.matches(skipped).count(), 1);
        assert_eq!(followed_target(&state).read(), LogRead::After(last));

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        assert_eq!(followed_target(&state).read(), LogRead::After(last));
        assert_eq!(log_view(&state).text(), text);
    }

    #[test]
    fn with_follow_off_a_restart_in_the_gap_keeps_follow_off() {
        let mut state = open_logs_with_two_lines();
        set_follow(&mut state, false);

        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
        );

        let view = log_view(&state);
        assert!(!view.is_following());
        assert_eq!(view.text(), "");
    }

    #[test]
    fn a_stopped_container_keeps_its_lines_and_starts_no_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED, false)),
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    /// The old stream failed before the answer came: the container simply
    /// ended, so its message says nothing any more.
    #[test]
    fn a_stopped_container_clears_the_error_of_the_broken_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream.clone(),
                error: ContainersError::new(
                    "error reading a body from connection",
                ),
            }),
        );

        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED, false)),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
        assert_eq!(logs.lines().text(), "one\ntwo");
        assert_eq!(followed_target(&state), stream);
    }

    /// No new stream starts for a stopped container, so its old stream keeps
    /// its key and its late failure must not bring the message back.
    #[test]
    fn a_late_failure_of_the_stream_of_a_stopped_container_is_dropped() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);
        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED, false)),
        );

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream,
                error: ContainersError::new(
                    "error reading a body from connection",
                ),
            }),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    /// A failed answer says nothing about the container, so the message stays.
    #[test]
    fn a_failed_answer_keeps_the_error_of_the_broken_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream.clone(),
                error: ContainersError::new("connection closed"),
            }),
        );

        run_checked(&mut state, Err(ContainersError::new("No such container")));

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("connection closed"));

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("a later failure"),
            }),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("a later failure"));
    }

    /// The same run still runs, but its lines have no time to go on from, so
    /// no stream starts and the message of the broken one is still true.
    #[test]
    fn a_running_container_with_lines_without_a_time_keeps_the_error() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        lines_arrived(&mut state, &["no time"], None);
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            }),
        );

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("connection closed"));
    }

    /// The container runs again: its new stream's failures count again.
    #[test]
    fn a_start_after_a_stop_shows_the_failures_of_the_new_stream() {
        let mut state = open_logs_with_two_lines();
        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED, false)),
        );

        container_started(&mut state, 0);
        let stream = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            }),
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("connection closed"));
    }

    #[test]
    fn a_gone_container_keeps_its_lines_and_starts_no_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        run_checked(&mut state, Err(ContainersError::new("No such container")));

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    /// Nothing answers the check with another check or stream: the only thing
    /// that asks again is the next reconnect.
    #[test]
    fn an_answer_about_the_run_asks_the_engine_for_nothing_more() {
        let mut state = open_logs_with_two_lines();
        let target = followed_target(&state);

        let answers = [
            Err(ContainersError::new("No such container")),
            Ok(fixtures::container_run(RUN_STARTED, false)),
            Ok(fixtures::container_run(RUN_STARTED, true)),
        ];
        for result in answers {
            let task = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                    target: target.clone(),
                    result,
                }),
            );

            assert!(effects(task).is_empty());
        }
    }

    #[test]
    fn lines_without_a_time_stay_and_are_not_read_again() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        lines_arrived(&mut state, &["no time"], None);
        let stream = followed_target(&state);

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "no time");
    }

    /// Nothing arrived, so nothing can repeat; a stream that read the latest
    /// run only stays that way.
    #[test]
    fn with_no_lines_yet_the_new_stream_reads_what_the_old_one_read() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        let first = followed_target(&state);

        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));
        let second = followed_target(&state);

        container_started(&mut state, 0);
        run_read(&mut state, RUN_STARTED_AGAIN);
        let after_a_start = followed_target(&state);
        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
        );

        assert_ne!(first, second);
        assert_eq!(second.read(), LogRead::LastLines);
        assert_ne!(followed_target(&state), after_a_start);
        assert_eq!(followed_target(&state).read(), LogRead::LatestRun);
    }

    /// The logs opened while the engine was away: the stream failed before it
    /// could read its run. The engine is back, so they are read now.
    #[test]
    fn logs_that_failed_before_reading_their_run_are_read_again() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let first = followed_target(&state);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogsFailed {
                target: first.clone(),
                error: ContainersError::new("connection refused"),
            }),
        );

        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED, false)),
        );

        assert_ne!(followed_target(&state), first);
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    #[test]
    fn a_start_forgets_the_run_and_the_position_of_the_old_stream() {
        let mut state = open_logs_with_two_lines();

        container_started(&mut state, 0);
        run_read(&mut state, RUN_STARTED_AGAIN);
        lines_arrived(&mut state, &["fresh"], None);
        run_checked(
            &mut state,
            Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
        );

        assert_eq!(log_view(&state).text(), "fresh");
        assert_eq!(
            followed_target(&state).read(),
            LogRead::LatestRun,
            "the old position must not be used; no new one came"
        );
    }

    /// The answer of an older stream: asked before the start event that came
    /// while it was on its way.
    #[test]
    fn an_answer_about_an_older_stream_changes_nothing() {
        let mut state = open_logs_with_two_lines();
        let asked_for = followed_target(&state);
        container_started(&mut state, 0);
        lines_arrived(&mut state, &["fresh"], None);
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
            }),
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "fresh");
    }

    #[test]
    fn two_reconnects_in_a_row_give_one_new_stream() {
        let mut state = open_logs_with_two_lines();
        let asked_for = followed_target(&state);

        for _ in 0..2 {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                    target: asked_for.clone(),
                    result: Ok(fixtures::container_run(RUN_STARTED, true)),
                }),
            );
        }

        let stream = followed_target(&state);
        assert_eq!(log_view(&state).text(), "one\ntwo");
        assert_eq!(stream.read(), LogRead::After(seen()));
        let once = {
            let mut once = open_logs_with_two_lines();
            run_checked(
                &mut once,
                Ok(fixtures::container_run(RUN_STARTED, true)),
            );
            followed_target(&once)
        };
        assert_eq!(stream, once);
    }

    #[test]
    fn an_answer_after_another_row_was_selected_is_dropped() {
        let mut state = open_logs_with_two_lines();
        let asked_for = followed_target(&state);
        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
            }),
        );

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(followed_target(&state), stream);
    }

    #[test]
    fn answers_after_the_tab_or_the_mode_was_left_are_dropped() {
        let leave = [
            ContainersIntent::SelectInspectorTab(InspectorTab::Overview),
            ContainersIntent::Leave,
        ];

        for intent in leave {
            let mut state = open_logs_with_two_lines();
            let asked_for = followed_target(&state);
            let _ = reduce_containers(&mut state, intent);

            let late = [
                ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                    target: asked_for.clone(),
                    result: Ok(fixtures::container_run(RUN_STARTED, true)),
                }),
                ContainersIntent::Inspector(InspectorIntent::LogRunRead {
                    target: asked_for.clone(),
                    run: fixtures::container_run(RUN_STARTED, true),
                }),
            ];
            for intent in late {
                let task = reduce_containers(&mut state, intent);

                assert!(effects(task).is_empty());
            }

            assert_eq!(followed(&state), None);
        }
    }

    /// A run read by a stream that was replaced says nothing about the lines
    /// on screen, so the check still sees the run they belong to.
    #[test]
    fn a_late_run_of_an_older_stream_is_not_taken() {
        let mut state = open_logs_with_two_lines();
        let old_stream = followed_target(&state);
        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogRunRead {
                target: old_stream,
                run: fixtures::container_run(RUN_STARTED_AGAIN, true),
            }),
        );
        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    #[test]
    fn logs_opened_anew_after_a_reconnect_begin_with_the_last_lines() {
        let mut state = open_logs_with_two_lines();
        run_checked(&mut state, Ok(fixtures::container_run(RUN_STARTED, true)));

        let _ = select_tab(&mut state, InspectorTab::Overview);
        let _ = select_tab(&mut state, InspectorTab::Logs);

        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
        assert_eq!(log_line_count(&state), 0);
    }

    /// `Unreachable` only starts a search: what the search finds decides.
    #[test]
    fn an_unreachable_engine_keeps_the_logs_while_it_is_searched_for() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    /// The search still finds the engine: the logs stay, and the reconnect
    /// that follows takes care of them.
    #[test]
    fn an_engine_found_again_keeps_the_logs_for_its_reconnect() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let effects =
            effects(reduce_containers(&mut state, reconnected(&docker())));

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
        assert!(effects.contains(&ContainersEffect::Inspector(
            InspectorEffect::CheckLogRun(stream)
        )));
    }

    /// The only engine goes away: the no-engine page, no logs and no stream.
    /// A late answer changes nothing. It comes back by Check again or by
    /// entering Containers mode again, with a new watch that sends no reconnect.
    /// The lists load anew and the row from before is selected again; the tab
    /// is still Logs, so that row's last lines are read from the start.
    #[test]
    fn the_only_engine_lost_and_found_again_brings_back_its_row_and_reads_its_logs_afresh()
     {
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        let position = state.shown_position(1);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::SelectRow(position),
        );
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        lines_arrived(&mut state, &["one"], Some(seen()));
        let asked_for = followed_target(&state);
        assert_eq!(followed(&state), Some(container_id(1)));

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );

        assert!(state.engines().is_empty());
        assert_eq!(followed(&state), None);
        let task = reduce_containers(
            &mut state,
            ContainersIntent::Inspector(InspectorIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED, true)),
            }),
        );
        assert!(effects(task).is_empty());
        assert_eq!(followed(&state), None);

        let _ = reduce_containers(&mut state, ContainersIntent::CheckAgain);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        assert_eq!(followed(&state), None);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );

        assert_eq!(state.inspector_tab(), InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
        assert_eq!(log_line_count(&state), 0);
    }

    /// Press the link to `target` in the inspector.
    fn follow(
        state: &mut ContainersState,
        target: LinkTarget,
    ) -> Vec<ContainersEffect> {
        let pressed = InspectorIntent::LinkPressed(target);

        effects(reduce_containers(
            state,
            ContainersIntent::Inspector(pressed),
        ))
    }

    fn busybox_id(state: &ContainersState) -> String {
        let engine = state.open_engine().expect("docker engine");
        engine
            .images()
            .items()
            .iter()
            .find(|image| image.reference() == Some("busybox:1.37"))
            .map(|image| image.id().to_string())
            .expect("busybox image")
    }

    fn busybox_key(state: &ContainersState) -> String {
        let engine = state.open_engine().expect("docker engine");
        engine
            .images()
            .items()
            .iter()
            .find(|image| image.reference() == Some("busybox:1.37"))
            .map(|image| image.row_key().to_string())
            .expect("busybox image")
    }

    /// The tags of the `resource` page of Docker, as the user reads them.
    fn tags_of(state: &ContainersState, resource: ResourceKind) -> Vec<String> {
        let page = Place::new(docker().address().clone(), resource);
        let keys = keys(resource);

        state
            .table_search(&page)
            .tags()
            .iter()
            .map(|tag| tag.label(&keys))
            .collect()
    }

    /// The tree of the Inspect tab of the selected container, with `json` as
    /// the engine's answer.
    fn inspected_tree(state: &mut ContainersState, json: &str) -> InspectTree {
        let id = state.selected_id().expect("a selected row").to_string();
        let _ = reduce_containers(
            state,
            ContainersIntent::SelectInspectorTab(InspectorTab::Inspect),
        );
        let _ = reduce_containers(
            state,
            ContainersIntent::Inspector(InspectorIntent::InspectLoaded {
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Containers,
                    &id,
                ),
                result: Ok(json.to_string()),
            }),
        );

        let vm = InspectorTabsViewModel::from_state(
            state.inspector(),
            &state.inspector_ctx(),
        )
        .expect("a selected container");
        let TabContent::Inspect(InspectContent::Loaded(loaded)) = vm.content
        else {
            panic!("expected the loaded inspect");
        };

        loaded.tree.expect("the tree")
    }

    #[test]
    fn a_link_in_the_inspect_tree_opens_its_item_through_follow_link() {
        let mut state = loaded_state();
        let busybox = busybox_id(&state);
        let key = busybox_key(&state);
        let target = LinkTarget::Item(ResourceKind::Images, key.clone());

        let tree = inspected_tree(
            &mut state,
            &format!("{{ \"Image\": \"{busybox}\" }}"),
        );
        let effects = follow(&mut state, target.clone());

        assert_eq!(
            tree.general,
            vec![DataRow::with_parts(
                0,
                "Image",
                [ValuePart::link(busybox.clone(), target)]
            )]
        );
        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Images
            ))
        );
        assert_eq!(state.selected_id(), Some(key.as_str()));
        assert_eq!(state.inspector_tab(), InspectorTab::Overview);
    }

    #[test]
    fn a_compose_label_in_the_inspect_tree_opens_the_filter_through_follow_link()
     {
        let mut state = loaded_state();
        let target = LinkTarget::ComposeProject(String::from("otty-fx"));

        let tree = inspected_tree(
            &mut state,
            "{ \"Config\": { \"Labels\": { \"com.docker.compose.project\": \
             \"otty-fx\" } } }",
        );
        let effects = follow(&mut state, target.clone());

        assert_eq!(
            tree.sections[0].rows,
            vec![
                DataRow::heading(0, "Labels"),
                DataRow::with_parts(
                    1,
                    "com.docker.compose.project",
                    [ValuePart::link("otty-fx", target)]
                ),
            ]
        );
        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Compose:otty-fx"]
        );
    }

    #[test]
    fn a_port_in_the_inspect_tree_opens_its_page_through_follow_link() {
        let mut state = loaded_state();
        let target = LinkTarget::Url(String::from("http://localhost:18081"));
        let place = state.place().cloned();

        let tree = inspected_tree(
            &mut state,
            "{ \"NetworkSettings\": { \"Ports\": { \"8080/tcp\": [{ \
             \"HostIp\": \"0.0.0.0\", \"HostPort\": \"18081\" }] } } }",
        );
        let effects = follow(&mut state, target.clone());

        assert_eq!(
            tree.sections[0].rows[4],
            DataRow::with_parts(
                3,
                "HostPort",
                [ValuePart::link("18081", target)]
            )
        );
        assert_eq!(
            effects,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18081"
            ))]
        );
        assert_eq!(state.place().cloned(), place);
    }
}
