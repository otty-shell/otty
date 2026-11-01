use iced::Task;
use otty_docker::{EngineAddress, EngineInfo};

use super::event::{DockerEffect, DockerEvent, DockerIntent};
use super::state::{DockerState, Search};
use super::types::{LoadedList, ResourceKind};
use super::{action_flow, form_flow};

/// Reduce a Docker intent into state changes and effects. Engine work
/// (search, list loads, inspect) is only asked for while Docker mode is
/// active (spec Decision 14); the reducer itself does no I/O. After every
/// intent the inspector follows the selection: see [`sync_inspector`].
pub(crate) fn reduce(
    state: &mut DockerState,
    intent: DockerIntent,
) -> Task<DockerEvent> {
    let task = reduce_intent(state, intent);

    let inspector = sync_inspector(state);
    Task::batch([task, inspector])
}

fn reduce_intent(
    state: &mut DockerState,
    intent: DockerIntent,
) -> Task<DockerEvent> {
    match intent {
        DockerIntent::Enter => {
            state.set_active(true);
            state.start_search();
            effect(DockerEffect::Discover)
        },
        DockerIntent::Leave => {
            state.set_active(false);
            Task::none()
        },
        DockerIntent::DiscoveryFinished(discovery) => {
            state.finish_search(discovery);
            load_every_engine(state)
        },
        DockerIntent::ListsLoaded { engine, lists } => {
            if !state.apply_lists(&engine, lists) {
                log::debug!("docker lists of a gone engine dropped: {engine}");
            }
            Task::none()
        },
        DockerIntent::Open(place) => {
            state.open(place);
            Task::none()
        },
        DockerIntent::ClearComposeProject => {
            state.clear_compose_project();
            Task::none()
        },
        DockerIntent::SelectRow(index) => {
            let _ = state.select_shown_row(index);
            Task::none()
        },
        DockerIntent::FilterChanged(filter) => {
            state.set_filter(filter);
            Task::none()
        },
        DockerIntent::Refresh => load_every_engine(state),
        DockerIntent::CheckAgain => {
            state.start_search();
            effect(DockerEffect::Discover)
        },
        DockerIntent::CopyCommand(command) => {
            effect(DockerEffect::CopyToClipboard(command))
        },
        DockerIntent::OpenLink(url) => effect(DockerEffect::OpenUrl(url)),
        DockerIntent::SelectInspectorTab(tab) => {
            state.select_inspector_tab(tab);
            Task::none()
        },
        DockerIntent::LogLines { target, lines } => {
            if !state.inspector_mut().push_log_lines(&target, lines) {
                log::debug!("docker log lines of an unfollowed container");
            }
            Task::none()
        },
        DockerIntent::LogsFailed { target, message } => {
            if !state.inspector_mut().fail_logs(&target, message) {
                log::debug!("docker log error of an unfollowed container");
            }
            Task::none()
        },
        DockerIntent::LogAction(action) => {
            state.inspector_mut().perform_log_action(action);
            Task::none()
        },
        DockerIntent::LogFollowToggled(is_following) => {
            state.inspector_mut().set_log_following(is_following);
            Task::none()
        },
        DockerIntent::InspectLoaded { target, result } => {
            if !state.inspector_mut().finish_inspect(&target, result) {
                log::debug!("docker inspect of a row no longer shown");
            }
            Task::none()
        },
        DockerIntent::InspectAction(action) => {
            state.inspector_mut().perform_inspect_action(action);
            Task::none()
        },
        DockerIntent::ListChanged { engine, resource } => {
            reload_changed_list(state, &engine, resource)
        },
        DockerIntent::EngineReconnected(engine) => {
            match active_engine_info(state, &engine) {
                Some(info) => effect(DockerEffect::LoadLists(info)),
                None => Task::none(),
            }
        },
        DockerIntent::EngineUnreachable(engine) => {
            search_after_lost_engine(state, &engine)
        },
        DockerIntent::ListLoaded { engine, list } => {
            apply_reloaded_list(state, &engine, list)
        },
        DockerIntent::Act(action) => action_flow::act(state, action),
        DockerIntent::OpenRowMenu { index, position } => {
            action_flow::open_row_menu(state, index, position)
        },
        DockerIntent::CloseRowMenu => {
            state.close_row_menu();
            Task::none()
        },
        DockerIntent::Prune => action_flow::prune(state),
        DockerIntent::ConfirmSwitchToggled(is_on) => {
            state.set_confirm_switch(is_on);
            Task::none()
        },
        DockerIntent::ConfirmAccepted => action_flow::accept(state),
        DockerIntent::ConfirmCancelled => {
            let _ = state.take_confirm();
            Task::none()
        },
        DockerIntent::ActionFinished { request, result } => {
            action_flow::finish(state, &request, result)
        },
        DockerIntent::DismissNotice => {
            state.dismiss_notice();
            Task::none()
        },
        DockerIntent::OpenCreate => form_flow::open(state),
        DockerIntent::FormChanged(field, value) => {
            if let Some(form) = state.form_mut() {
                form.set_value(field, value);
            }
            Task::none()
        },
        DockerIntent::FormSubmitted => form_flow::submit(state),
        DockerIntent::FormCancelled => form_flow::cancel(state),
        DockerIntent::PullProgressed(progress) => {
            if let Some(form) = state.form_mut() {
                form.set_progress(progress);
            }
            Task::none()
        },
        DockerIntent::PullFinished(result) => {
            form_flow::finish_pull(state, result)
        },
    }
}

/// Make the inspector match the state after an intent: follow the logs of
/// the selected container only while the Logs tab is open in active Docker
/// mode, and load the inspect JSON once per selection when the Inspect tab
/// is open. A tab, selection or mode change stops the old logs.
fn sync_inspector(state: &mut DockerState) -> Task<DockerEvent> {
    let log_target = state.wanted_log_target();
    state.inspector_mut().follow_logs(log_target);

    let Some((engine, target)) = state.wanted_inspect_target() else {
        return Task::none();
    };
    if !state.inspector_mut().start_inspect(target.clone()) {
        return Task::none();
    }

    effect(DockerEffect::LoadInspect { engine, target })
}

/// An engine event changed `resource`: reload it while Docker mode is
/// active. "Used by" of images, volumes and networks is read from the
/// containers shown now; a container reload brings its own list.
fn reload_changed_list(
    state: &DockerState,
    address: &EngineAddress,
    resource: ResourceKind,
) -> Task<DockerEvent> {
    let Some(info) = active_engine_info(state, address) else {
        return Task::none();
    };

    let containers = match resource {
        ResourceKind::Containers => Vec::new(),
        _ => state
            .engine(address)
            .map(|engine| engine.containers().items().to_vec())
            .unwrap_or_default(),
    };

    effect(DockerEffect::LoadList {
        info,
        resource,
        containers,
    })
}

/// Take a reloaded list. A new container list also reloads images, volumes
/// and networks with it, so their "used by" stays right; a failed one shows
/// its error and leaves the other lists as they are.
fn apply_reloaded_list(
    state: &mut DockerState,
    address: &EngineAddress,
    list: LoadedList,
) -> Task<DockerEvent> {
    let new_containers = match &list {
        LoadedList::Containers(Ok(containers)) => Some(containers.clone()),
        _ => None,
    };

    if !state.apply_list(address, list) {
        log::debug!("docker list of a gone engine dropped: {address}");
        return Task::none();
    }

    let (Some(containers), Some(info)) =
        (new_containers, active_engine_info(state, address))
    else {
        return Task::none();
    };

    Task::batch(
        [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ]
        .into_iter()
        .map(|resource| {
            effect(DockerEffect::LoadList {
                info: info.clone(),
                resource,
                containers: containers.clone(),
            })
        }),
    )
}

/// An engine stopped answering: search again, unless a search runs, so the
/// tree and the no-engine card show what is left (ticket 10).
fn search_after_lost_engine(
    state: &mut DockerState,
    address: &EngineAddress,
) -> Task<DockerEvent> {
    if active_engine_info(state, address).is_none()
        || state.search() == &Search::Searching
    {
        return Task::none();
    }

    state.start_search();
    effect(DockerEffect::Discover)
}

/// The engine at `address`, only while Docker mode is active.
fn active_engine_info(
    state: &DockerState,
    address: &EngineAddress,
) -> Option<EngineInfo> {
    if !state.is_active() {
        return None;
    }

    state.engine(address).map(|engine| engine.info().clone())
}

/// Ask to load the lists of every engine, only while Docker mode is
/// active: a search that finishes after the user left starts nothing.
fn load_every_engine(state: &DockerState) -> Task<DockerEvent> {
    if !state.is_active() {
        return Task::none();
    }

    Task::batch(
        state.engines().iter().map(|engine| {
            effect(DockerEffect::LoadLists(engine.info().clone()))
        }),
    )
}

fn effect(effect: DockerEffect) -> Task<DockerEvent> {
    Task::done(DockerEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use iced::widget::text_editor::Action;
    use otty_docker::{
        Container, EngineInfo, EngineKind, Image, Network, Volume, fixtures,
    };
    use otty_ui_kit::components::log_view::LogViewState;

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::inspector::{
        InspectLoad, InspectTarget, InspectorTab, LogTarget, MAX_LOG_LINES,
    };
    use crate::widgets::docker::state::{ListLoad, Search};
    use crate::widgets::docker::types::{
        LoadedList, LoadedLists, Place, ResourceKind,
    };

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

    fn effects(task: Task<DockerEvent>) -> Vec<DockerEffect> {
        collect_outputs(task)
            .into_iter()
            .filter_map(|event| match event {
                DockerEvent::Effect(effect) => Some(effect),
                DockerEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Docker mode active, both fixture engines found, the Docker engine's
    /// lists loaded.
    fn loaded_state() -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        state
    }

    fn open(state: &mut DockerState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, DockerIntent::Open(place));
    }

    fn selected_container(state: &DockerState) -> Option<&str> {
        state.open_engine()?.containers().selected_id()
    }

    #[test]
    fn checking_again_starts_a_new_search() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        assert!(matches!(state.search(), Search::Done(_)));

        let effects = effects(reduce(&mut state, DockerIntent::CheckAgain));

        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![DockerEffect::Discover]);
    }

    #[test]
    fn an_engine_started_meanwhile_is_found_by_checking_again() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        let _ = reduce(&mut state, DockerIntent::CheckAgain);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));

        assert_eq!(state.engines().len(), 1);
        assert_eq!(effects, vec![DockerEffect::LoadLists(docker())]);
    }

    #[test]
    fn copying_a_command_asks_for_the_clipboard() {
        let mut state = DockerState::default();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::CopyCommand(String::from(
                "sudo systemctl start docker",
            )),
        ));

        assert_eq!(
            effects,
            vec![DockerEffect::CopyToClipboard(String::from(
                "sudo systemctl start docker"
            ))]
        );
    }

    #[test]
    fn opening_a_link_asks_for_the_browser() {
        let mut state = DockerState::default();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::OpenLink(String::from("https://podman.io")),
        ));

        assert_eq!(
            effects,
            vec![DockerEffect::OpenUrl(String::from("https://podman.io"))]
        );
    }

    #[test]
    fn entering_docker_mode_searches_for_engines() {
        let mut state = DockerState::default();

        let effects = effects(reduce(&mut state, DockerIntent::Enter));

        assert!(state.is_active());
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![DockerEffect::Discover]);
    }

    #[test]
    fn found_engines_load_their_lists_and_open_the_first_engines_containers() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        ));

        assert_eq!(
            effects,
            vec![
                DockerEffect::LoadLists(docker()),
                DockerEffect::LoadLists(podman())
            ]
        );
        assert_eq!(state.engines().len(), 2);
        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Containers
            ))
        );
    }

    #[test]
    fn no_engine_work_starts_outside_docker_mode() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(&mut state, DockerIntent::Leave);

        let after_search = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));
        let after_refresh = effects(reduce(&mut state, DockerIntent::Refresh));

        assert!(!state.is_active());
        assert!(after_search.is_empty());
        assert!(after_refresh.is_empty());
        assert_eq!(state.engines().len(), 1);
    }

    #[test]
    fn no_engine_clears_the_engines_and_the_place() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        ));

        assert!(effects.is_empty());
        assert!(state.engines().is_empty());
        assert!(state.place().is_none());
        assert!(matches!(state.search(), Search::Done(_)));
    }

    #[test]
    fn a_new_search_keeps_the_lists_and_the_place_of_engines_found_again() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let engine = state.open_engine().expect("docker is still there");
        assert_eq!(engine.containers().items().len(), 6);
        assert_eq!(state.engines().len(), 1);
        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Images)
        );
    }

    #[test]
    fn loaded_lists_select_their_first_rows() {
        let state = loaded_state();

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            selected_container(&state),
            Some(engine.containers().items()[0].id())
        );
        assert_eq!(
            engine.images().selected_id(),
            Some(engine.images().items()[0].id())
        );
        assert_eq!(
            engine.volumes().selected_id(),
            Some(engine.volumes().items()[0].name())
        );
        assert_eq!(
            engine.networks().selected_id(),
            Some(engine.networks().items()[0].id())
        );
    }

    #[test]
    fn opening_each_resource_shows_it_and_clears_the_filter() {
        let mut state = loaded_state();

        // Containers are open first, so they come back last.
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
            ResourceKind::Containers,
        ] {
            let _ = reduce(
                &mut state,
                DockerIntent::FilterChanged(String::from("otty")),
            );

            open(&mut state, resource);

            assert_eq!(state.place().map(Place::resource), Some(resource));
            assert_eq!(state.filter(), "");
        }
    }

    #[test]
    fn opening_the_shown_place_again_keeps_the_filter() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));

        open(&mut state, ResourceKind::Containers);

        assert_eq!(state.filter(), "x");
    }

    #[test]
    fn opening_a_compose_project_keeps_the_containers_and_clears_the_filter() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));
        let project = Place::compose(docker().address().clone(), "otty-fx");

        let _ = reduce(&mut state, DockerIntent::Open(project));

        let place = state.place().expect("an open place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.compose_project(), Some("otty-fx"));
        assert_eq!(state.filter(), "");
    }

    #[test]
    fn clearing_the_compose_project_opens_all_containers() {
        let mut state = loaded_state();
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let _ = reduce(&mut state, DockerIntent::ClearComposeProject);

        let place = state.place().expect("an open place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.compose_project(), None);
    }

    #[test]
    fn clearing_without_a_project_changes_nothing() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));

        let _ = reduce(&mut state, DockerIntent::ClearComposeProject);

        assert_eq!(state.filter(), "x");
    }

    #[test]
    fn a_row_press_in_a_compose_project_selects_that_project_container() {
        let mut state = loaded_state();
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let _ = reduce(&mut state, DockerIntent::SelectRow(0));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-web-1"));
    }

    #[test]
    fn a_row_press_selects_that_row_of_the_filtered_table() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            DockerIntent::FilterChanged(String::from("exit")),
        );

        let _ = reduce(&mut state, DockerIntent::SelectRow(1));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit0"));
    }

    #[test]
    fn a_row_press_on_each_resource_selects_its_row() {
        let mut state = loaded_state();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            let _ = reduce(&mut state, DockerIntent::SelectRow(1));
        }

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.containers().selected(Container::id),
            engine.containers().items().get(1)
        );
        assert_eq!(
            engine.images().selected(Image::id),
            engine.images().items().get(1)
        );
        assert_eq!(
            engine.volumes().selected(Volume::name),
            engine.volumes().items().get(1)
        );
        assert_eq!(
            engine.networks().selected(Network::id),
            engine.networks().items().get(1)
        );
    }

    #[test]
    fn a_row_press_past_the_last_row_changes_nothing() {
        let mut state = loaded_state();
        let before = selected_container(&state).map(str::to_string);

        let _ = reduce(&mut state, DockerIntent::SelectRow(99));

        assert_eq!(selected_container(&state).map(str::to_string), before);
    }

    #[test]
    fn a_reload_keeps_the_selection_while_the_row_is_there() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::SelectRow(2));
        let selected = selected_container(&state).map(str::to_string);
        let mut reloaded = lists(EngineKind::Docker);
        if let Ok(containers) = reloaded.containers.as_mut() {
            containers.remove(0);
        }

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        assert_eq!(selected_container(&state).map(str::to_string), selected);
    }

    #[test]
    fn a_reload_without_the_selected_row_selects_the_first_row() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::SelectRow(2));
        let mut reloaded = lists(EngineKind::Docker);
        if let Ok(containers) = reloaded.containers.as_mut() {
            containers.remove(2);
        }
        let first = match &reloaded.containers {
            Ok(containers) => containers[0].id().to_string(),
            Err(_) => String::new(),
        };

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        assert_eq!(selected_container(&state), Some(first.as_str()));
    }

    #[test]
    fn a_failed_list_shows_its_error_and_keeps_the_other_lists() {
        let mut state = loaded_state();
        let mut reloaded = lists(EngineKind::Docker);
        reloaded.images = Err(String::from("engine said no"));

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.images().load(),
            &ListLoad::Failed(String::from("engine said no"))
        );
        assert_eq!(engine.containers().items().len(), 6);
        assert_eq!(engine.volumes().items().len(), 2);
        assert_eq!(engine.networks().items().len(), 4);
    }

    #[test]
    fn lists_of_an_engine_that_is_gone_are_dropped() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );

        assert_eq!(state.engines().len(), 1);
        assert!(state.engine(podman().address()).is_none());
    }

    #[test]
    fn refresh_in_docker_mode_reloads_every_engine() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, DockerIntent::Refresh));

        assert_eq!(
            effects,
            vec![
                DockerEffect::LoadLists(docker()),
                DockerEffect::LoadLists(podman())
            ]
        );
    }

    #[test]
    fn leaving_docker_mode_keeps_what_is_shown() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, DockerIntent::Leave));

        assert!(effects.is_empty());
        assert!(!state.is_active());
        assert_eq!(state.engines().len(), 2);
        assert!(state.place().is_some());
    }

    // Inspector tabs.

    fn select_tab(
        state: &mut DockerState,
        tab: InspectorTab,
    ) -> Vec<DockerEffect> {
        effects(reduce(state, DockerIntent::SelectInspectorTab(tab)))
    }

    fn container_id(index: usize) -> String {
        let resources = fixtures::resources(EngineKind::Docker);

        resources.containers()[index].id().to_string()
    }

    fn log_target(index: usize) -> LogTarget {
        LogTarget::new(docker(), &container_id(index))
    }

    fn followed(state: &DockerState) -> Option<String> {
        let logs = state.inspector().logs()?;

        Some(logs.target().container_id().to_string())
    }

    fn log_line_count(state: &DockerState) -> usize {
        state
            .inspector()
            .logs()
            .map_or(0, |logs| logs.lines().line_count())
    }

    fn push_lines(state: &mut DockerState, index: usize, lines: &[&str]) {
        let lines = lines.iter().map(|line| line.to_string()).collect();
        let _ = reduce(
            state,
            DockerIntent::LogLines {
                target: log_target(index),
                lines,
            },
        );
    }

    fn inspect_of(resource: ResourceKind, id: &str) -> DockerEffect {
        DockerEffect::LoadInspect {
            engine: docker(),
            target: InspectTarget::new(
                docker().address().clone(),
                resource,
                id,
            ),
        }
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
        assert_eq!(state.inspector().tab(), InspectorTab::Logs);
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

        let _ = reduce(&mut state, DockerIntent::SelectRow(1));

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }

    #[test]
    fn the_logs_stop_when_docker_mode_is_left_and_follow_again_on_return() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        let _ = reduce(&mut state, DockerIntent::Leave);
        let after_leave = followed(&state);
        let _ = reduce(&mut state, DockerIntent::Enter);

        assert_eq!(after_leave, None);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    #[test]
    fn a_reload_that_keeps_the_container_keeps_its_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one", "two"]);

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
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

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
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

        assert_eq!(state.inspector().tab(), InspectorTab::Overview);
        assert_eq!(followed(&state), None);
    }

    #[test]
    fn opening_another_list_goes_back_to_the_overview() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        open(&mut state, ResourceKind::Images);
        let on_images = state.inspector().tab();
        open(&mut state, ResourceKind::Containers);

        assert_eq!(on_images, InspectorTab::Overview);
        assert_eq!(state.inspector().tab(), InspectorTab::Overview);
        assert_eq!(followed(&state), None);
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
            let _ = reduce(
                &mut state,
                DockerIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                },
            );
        }

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(MAX_LOG_LINES, 5000);
        assert_eq!(logs.lines().line_count(), MAX_LOG_LINES);
        let text = logs.lines().text();
        assert!(text.starts_with("line 100\n"), "{}", &text[..20]);
        assert!(text.ends_with("line 5099"));
    }

    fn log_view(state: &DockerState) -> &LogViewState {
        state.inspector().logs().expect("followed logs").lines()
    }

    fn set_follow(state: &mut DockerState, is_on: bool) {
        let _ = reduce(state, DockerIntent::LogFollowToggled(is_on));
    }

    /// The Logs tab of the first container with two lines, Follow off.
    fn logs_without_follow() -> DockerState {
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

        let _ = reduce(
            &mut state,
            DockerIntent::LogAction(Action::Scroll { lines: -2 }),
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

        let _ = reduce(
            &mut state,
            DockerIntent::LogLines {
                target: log_target(0),
                lines,
            },
        );
        let waiting = log_view(&state).pending_count();
        set_follow(&mut state, true);

        assert_eq!(waiting, MAX_LOG_LINES);
        let lines = log_view(&state);
        assert_eq!(lines.line_count(), MAX_LOG_LINES);
        let text = lines.text();
        assert!(text.starts_with("line 100\n"), "{}", &text[..20]);
        assert!(text.ends_with("line 5099"));
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

        let _ = reduce(&mut state, DockerIntent::SelectRow(1));

        assert_eq!(followed(&state), Some(container_id(1)));
        assert!(log_view(&state).is_following());
    }

    #[test]
    fn follow_is_on_again_after_a_mode_change() {
        let mut state = logs_without_follow();

        let _ = reduce(&mut state, DockerIntent::Leave);
        let _ = reduce(&mut state, DockerIntent::Enter);

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
        let _ = reduce(
            &mut state,
            DockerIntent::LogsFailed {
                target: log_target(1),
                message: String::from("gone"),
            },
        );

        assert_eq!(log_line_count(&state), 0);
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    #[test]
    fn a_failed_log_stream_shows_the_engine_message_and_keeps_the_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["starting"]);

        let _ = reduce(
            &mut state,
            DockerIntent::LogsFailed {
                target: log_target(0),
                message: String::from("No such container"),
            },
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

        let other = effects(reduce(&mut state, DockerIntent::SelectRow(1)));
        let same = effects(reduce(&mut state, DockerIntent::SelectRow(1)));

        assert_eq!(
            other,
            vec![inspect_of(ResourceKind::Containers, &container_id(1))]
        );
        assert!(same.is_empty());
    }

    #[test]
    fn a_selection_on_the_overview_loads_when_inspect_opens() {
        let mut state = loaded_state();

        let on_overview =
            effects(reduce(&mut state, DockerIntent::SelectRow(1)));
        let opened = select_tab(&mut state, InspectorTab::Inspect);

        assert!(on_overview.is_empty());
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

        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: container_inspect(0),
                result: Ok(json.clone()),
            },
        );
        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: container_inspect(1),
                result: Err(String::from("late")),
            },
        );

        let inspect = state.inspector().inspect().expect("inspect");
        assert_eq!(inspect.target(), &container_inspect(0));
        let InspectLoad::Loaded(shown) = inspect.load() else {
            panic!("expected the json, got {:?}", inspect.load());
        };
        assert_eq!(shown.text(), json);
    }

    #[test]
    fn a_failed_inspect_shows_the_engine_message() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: container_inspect(0),
                result: Err(String::from("No such container")),
            },
        );

        let inspect = state.inspector().inspect().expect("inspect");
        assert!(matches!(
            inspect.load(),
            InspectLoad::Failed(message) if message == "No such container"
        ));
    }

    #[test]
    fn nothing_is_inspected_outside_docker_mode() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Leave);

        let effects = select_tab(&mut state, InspectorTab::Inspect);

        assert!(effects.is_empty());
        assert!(state.inspector().inspect().is_none());
    }

    // Live updates from engine events (ticket 16).

    fn changed(resource: ResourceKind) -> DockerIntent {
        DockerIntent::ListChanged {
            engine: docker().address().clone(),
            resource,
        }
    }

    fn docker_containers() -> Vec<Container> {
        fixtures::resources(EngineKind::Docker)
            .containers()
            .to_vec()
    }

    #[test]
    fn a_container_event_reloads_the_container_list() {
        let mut state = loaded_state();

        let effects =
            effects(reduce(&mut state, changed(ResourceKind::Containers)));

        assert_eq!(
            effects,
            vec![DockerEffect::LoadList {
                info: docker(),
                resource: ResourceKind::Containers,
                containers: Vec::new(),
            }]
        );
    }

    #[test]
    fn an_image_volume_or_network_event_reloads_that_list_with_the_shown_containers()
     {
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ] {
            let mut state = loaded_state();

            let effects = effects(reduce(&mut state, changed(resource)));

            assert_eq!(
                effects,
                vec![DockerEffect::LoadList {
                    info: docker(),
                    resource,
                    containers: docker_containers(),
                }]
            );
        }
    }

    #[test]
    fn events_start_no_engine_work_outside_docker_mode() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Leave);
        let docker_address = docker().address().clone();

        let intents = [
            changed(ResourceKind::Containers),
            changed(ResourceKind::Images),
            DockerIntent::EngineReconnected(docker_address.clone()),
            DockerIntent::EngineUnreachable(docker_address),
        ];

        for intent in intents {
            assert!(effects(reduce(&mut state, intent)).is_empty());
        }
        assert!(matches!(state.search(), Search::Done(_)));
    }

    #[test]
    fn events_of_an_unknown_engine_are_dropped() {
        let mut state = loaded_state();
        let unknown = otty_docker::EngineAddress::Tcp(String::from("h:1"));

        let intents = [
            DockerIntent::ListChanged {
                engine: unknown.clone(),
                resource: ResourceKind::Containers,
            },
            DockerIntent::EngineReconnected(unknown.clone()),
            DockerIntent::EngineUnreachable(unknown),
        ];

        for intent in intents {
            assert!(effects(reduce(&mut state, intent)).is_empty());
        }
        assert!(matches!(state.search(), Search::Done(_)));
    }

    #[test]
    fn a_new_container_list_reloads_images_volumes_and_networks_with_it() {
        let mut state = loaded_state();
        let mut containers = docker_containers();
        containers.truncate(2);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Ok(containers.clone())),
            },
        ));

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(engine.containers().items(), containers.as_slice());
        assert_eq!(
            effects,
            vec![
                DockerEffect::LoadList {
                    info: docker(),
                    resource: ResourceKind::Images,
                    containers: containers.clone(),
                },
                DockerEffect::LoadList {
                    info: docker(),
                    resource: ResourceKind::Volumes,
                    containers: containers.clone(),
                },
                DockerEffect::LoadList {
                    info: docker(),
                    resource: ResourceKind::Networks,
                    containers,
                },
            ]
        );
    }

    #[test]
    fn a_failed_container_reload_shows_its_error_and_keeps_the_other_lists() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Err(String::from(
                    "unknown variant `unknown`",
                ))),
            },
        ));

        assert!(effects.is_empty());
        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.containers().load(),
            &ListLoad::Failed(String::from("unknown variant `unknown`"))
        );
        assert_eq!(engine.images().items().len(), 3);
        assert_eq!(engine.volumes().items().len(), 2);
        assert_eq!(engine.networks().items().len(), 4);
        let main = crate::widgets::docker::model::main_area(&state, 0);
        let crate::widgets::docker::model::MainViewModel::Table(table) = main
        else {
            panic!("the containers table is shown");
        };
        assert_eq!(table.error, Some("unknown variant `unknown`"));
    }

    #[test]
    fn a_reloaded_image_volume_or_network_list_replaces_only_that_list() {
        let mut state = loaded_state();
        let resources = fixtures::resources(EngineKind::Docker);
        let one_image = vec![resources.images()[0].clone()];
        let one_volume = vec![resources.volumes()[0].clone()];
        let one_network = vec![resources.networks()[0].clone()];

        let mut all_effects = Vec::new();
        for list in [
            LoadedList::Images(Ok(one_image.clone())),
            LoadedList::Volumes(Ok(one_volume.clone())),
            LoadedList::Networks(Ok(one_network.clone())),
        ] {
            all_effects.extend(effects(reduce(
                &mut state,
                DockerIntent::ListLoaded {
                    engine: docker().address().clone(),
                    list,
                },
            )));
        }

        assert!(all_effects.is_empty());
        let engine = state.open_engine().expect("docker engine");
        assert_eq!(engine.containers().items().len(), 6);
        assert_eq!(engine.images().items(), one_image.as_slice());
        assert_eq!(engine.volumes().items(), one_volume.as_slice());
        assert_eq!(engine.networks().items(), one_network.as_slice());
    }

    #[test]
    fn a_reloaded_list_of_a_gone_engine_is_dropped() {
        let mut state = loaded_state();
        let gone = otty_docker::EngineAddress::Tcp(String::from("h:1"));

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ListLoaded {
                engine: gone,
                list: LoadedList::Containers(Ok(Vec::new())),
            },
        ));

        assert!(effects.is_empty());
        let engine = state.open_engine().expect("docker engine");
        assert_eq!(engine.containers().items().len(), 6);
    }

    #[test]
    fn a_reconnected_engine_reloads_every_list() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::EngineReconnected(podman().address().clone()),
        ));

        assert_eq!(effects, vec![DockerEffect::LoadLists(podman())]);
    }

    #[test]
    fn an_unreachable_engine_starts_a_new_search() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::EngineUnreachable(docker().address().clone()),
        ));

        assert_eq!(effects, vec![DockerEffect::Discover]);
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(state.engines().len(), 2);
    }

    #[test]
    fn an_unreachable_engine_during_a_search_starts_no_second_one() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::CheckAgain);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::EngineUnreachable(docker().address().clone()),
        ));

        assert!(effects.is_empty());
    }

    #[test]
    fn an_event_reload_keeps_the_logs_of_a_kept_container_and_stops_a_gone_one()
    {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["ready"]);
        let reload = |containers: Vec<Container>| DockerIntent::ListLoaded {
            engine: docker().address().clone(),
            list: LoadedList::Containers(Ok(containers)),
        };

        let _ = reduce(&mut state, reload(docker_containers()));
        let kept = (followed(&state), log_line_count(&state));
        let _ = reduce(&mut state, reload(docker_containers()[1..].to_vec()));

        assert_eq!(kept, (Some(container_id(0)), 1));
        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }
}
