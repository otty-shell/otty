use iced::Task;
use otty_containers::{Discovery, EngineAddress, EngineInfo};

use super::event::{ContainersEffect, ContainersEvent, ContainersIntent};
use super::inspector_tabs::refresh_inspect_tree;
use super::links::LinkTarget;
use super::state::{ContainersState, Search};
use super::types::{LoadedList, ResourceKind};
use super::{action_flow, form_flow, run_flow};

/// Reduce a Containers intent into state changes and effects. Engine work
/// (search, list loads, inspect) is only asked for while Containers mode is
/// active (spec Decision 14); the reducer itself does no I/O. After every
/// intent the inspector follows the selection: see [`sync_inspector`].
pub(crate) fn reduce(
    state: &mut ContainersState,
    intent: ContainersIntent,
) -> Task<ContainersEvent> {
    let task = reduce_intent(state, intent);

    let inspector = sync_inspector(state);
    Task::batch([task, inspector])
}

fn reduce_intent(
    state: &mut ContainersState,
    intent: ContainersIntent,
) -> Task<ContainersEvent> {
    match intent {
        ContainersIntent::FromPage { page, intent } => {
            if state.place() == Some(&page) {
                reduce_intent(state, *intent)
            } else {
                log::debug!(
                    "containers intent of a page no longer open dropped"
                );
                Task::none()
            }
        },
        ContainersIntent::Enter => {
            state.set_active(true);
            state.start_search();

            // The timer did not run in the other mode, so the open form asks
            // about its folders now rather than in up to 2 seconds.
            let folders = run_flow::recheck_folders(state);
            Task::batch([effect(ContainersEffect::Discover), folders])
        },
        ContainersIntent::Leave => {
            state.set_active(false);
            state.close_search_suggestions();
            Task::none()
        },
        ContainersIntent::RightPanelHidden(is_hidden) => {
            state.set_right_panel_hidden(is_hidden);
            Task::none()
        },
        ContainersIntent::DiscoveryFinished(discovery) => {
            state.finish_search(discovery);
            load_every_engine(state)
        },
        ContainersIntent::EngineSearchTick => search_behind_the_card(state),
        ContainersIntent::BackgroundSearchFinished(discovery) => {
            finish_search_behind_the_card(state, discovery)
        },
        ContainersIntent::ListsLoaded { engine, lists } => {
            if !state.apply_lists(&engine, lists) {
                log::debug!(
                    "containers lists of a gone engine dropped: {engine}"
                );
            }
            Task::none()
        },
        ContainersIntent::Open(place) => {
            let closed = run_flow::close(state);
            state.open(place);
            closed
        },
        ContainersIntent::ToggleEngine(engine) => {
            state.toggle_engine(engine);
            Task::none()
        },
        ContainersIntent::OpenComposeProject(index) => {
            let _ = state.open_compose_project_of_row(index);
            Task::none()
        },
        ContainersIntent::OpenPortLink { row, link } => {
            match state.port_url_of_row(row, link) {
                Some(url) => effect(ContainersEffect::OpenUrl(url)),
                None => Task::none(),
            }
        },
        ContainersIntent::SortBy(column) => {
            state.sort_by(column);
            Task::none()
        },
        ContainersIntent::SelectRow(index) => {
            if state.select_shown_row(index) {
                effect(ContainersEffect::RevealInspector)
            } else {
                Task::none()
            }
        },
        ContainersIntent::TagSearch(action) => {
            state.perform_search(action);
            Task::none()
        },
        ContainersIntent::Refresh => load_every_engine(state),
        ContainersIntent::CheckAgain => {
            state.start_search();
            effect(ContainersEffect::Discover)
        },
        ContainersIntent::CopyCommand(command) => {
            effect(ContainersEffect::CopyToClipboard(command))
        },
        ContainersIntent::OpenLink(url) => {
            effect(ContainersEffect::OpenUrl(url))
        },
        ContainersIntent::FollowLink(target) => follow_link(state, target),
        ContainersIntent::SelectInspectorTab(tab) => {
            state.select_inspector_tab(tab);
            Task::none()
        },
        ContainersIntent::LogLines {
            target,
            lines,
            position,
        } => {
            let inspector = state.inspector_mut();
            if !inspector.push_log_lines(&target, lines, position) {
                log::debug!("containers log lines of an unfollowed container");
            }
            Task::none()
        },
        ContainersIntent::LogRunRead { target, run } => {
            if !state.inspector_mut().set_log_run(&target, run) {
                log::debug!("containers log run of an unfollowed container");
            }
            Task::none()
        },
        ContainersIntent::LogRunChecked { target, result } => {
            let run_now = result
                .inspect_err(|err| log::debug!("containers log run: {err}"))
                .ok();
            let inspector = state.inspector_mut();
            if !inspector.settle_logs_after_reconnect(&target, run_now) {
                log::debug!(
                    "containers log run check of an older stream dropped"
                );
            }
            Task::none()
        },
        ContainersIntent::LogsFailed { target, error } => {
            if !state.inspector_mut().fail_logs(&target, error) {
                log::debug!("containers log error of an old stream dropped");
            }
            Task::none()
        },
        ContainersIntent::LogAction(action) => {
            state.inspector_mut().perform_log_action(action);
            Task::none()
        },
        ContainersIntent::LogFollowToggled(is_following) => {
            state.inspector_mut().set_log_following(is_following);
            Task::none()
        },
        ContainersIntent::InspectLoaded { target, result } => {
            if !state.inspector_mut().finish_inspect(&target, result) {
                log::debug!("containers inspect of a row no longer shown");
            }
            Task::none()
        },
        ContainersIntent::InspectAction(action) => {
            state.inspector_mut().perform_inspect_action(action);
            Task::none()
        },
        ContainersIntent::InspectSectionToggled(title) => {
            state.inspector_mut().toggle_inspect_section(&title);
            Task::none()
        },
        ContainersIntent::InspectShowEmptyToggled(is_on) => {
            state.inspector_mut().set_shows_empty(is_on);
            Task::none()
        },
        ContainersIntent::InspectRawJsonToggled(is_on) => {
            state.inspector_mut().set_raw_json(is_on);
            Task::none()
        },
        ContainersIntent::CopyInspectJson => {
            match state.inspector().inspect_json_text() {
                Some(json) => effect(ContainersEffect::CopyToClipboard(json)),
                None => Task::none(),
            }
        },
        ContainersIntent::ListChanged { engine, resource } => {
            reload_changed_list(state, &engine, resource)
        },
        ContainersIntent::ContainerStarted { engine, id } => {
            state.inspector_mut().restart_logs(&engine, &id);
            Task::none()
        },
        ContainersIntent::EngineReconnected(engine) => {
            let lists = match active_engine_info(state, &engine) {
                Some(info) => effect(ContainersEffect::LoadLists(info)),
                None => Task::none(),
            };

            Task::batch([lists, check_open_logs(state, &engine)])
        },
        ContainersIntent::EngineUnreachable(engine) => {
            search_after_lost_engine(state, &engine)
        },
        ContainersIntent::ListLoaded { engine, list } => {
            apply_reloaded_list(state, &engine, list)
        },
        ContainersIntent::Act(action) => action_flow::act(state, action),
        ContainersIntent::OpenRowMenu { index, position } => {
            action_flow::select_and_open_row_menu(state, index, position)
        },
        ContainersIntent::RowMenuPressed { index, position } => {
            action_flow::open_row_menu(state, index, position)
        },
        ContainersIntent::CloseRowMenu => {
            state.close_row_menu();
            Task::none()
        },
        ContainersIntent::Prune => action_flow::prune(state),
        ContainersIntent::ConfirmSwitchToggled(is_on) => {
            state.set_confirm_switch(is_on);
            Task::none()
        },
        ContainersIntent::ConfirmAccepted => action_flow::accept(state),
        ContainersIntent::ConfirmCancelled => {
            let _ = state.take_confirm();
            Task::none()
        },
        ContainersIntent::ActionFinished { request, result } => {
            action_flow::finish(state, &request, result)
        },
        ContainersIntent::DismissNotice => {
            state.dismiss_notice();
            Task::none()
        },
        ContainersIntent::OpenCreate => form_flow::open(state),
        ContainersIntent::FormChanged(field, value) => {
            if let Some(form) = state.form_mut() {
                form.set_value(field, value);
            }
            Task::none()
        },
        ContainersIntent::FormSubmitted => form_flow::submit(state),
        ContainersIntent::FormCancelled => form_flow::cancel(state),
        ContainersIntent::PullProgressed(progress) => {
            if run_flow::is_pulling(state) {
                run_flow::pull_progressed(state, progress);
            } else if let Some(form) = state.form_mut() {
                form.set_progress(progress);
            }
            Task::none()
        },
        ContainersIntent::PullFinished(result) => {
            if run_flow::is_pulling(state) {
                run_flow::finish_pull(state, result)
            } else {
                form_flow::finish_pull(state, result)
            }
        },
        ContainersIntent::RunEdited(edit) => run_flow::edit(state, edit),
        ContainersIntent::RunEngineChosen(engine) => {
            run_flow::choose_engine(state, &engine)
        },
        ContainersIntent::RunSubmitted => run_flow::submit(state),
        ContainersIntent::RunCancelled => run_flow::close(state),
        ContainersIntent::RunNoticeDismissed => {
            if let Some(form) = state.run_form_mut() {
                form.dismiss_notice();
            }
            Task::none()
        },
        ContainersIntent::RunFinished { engine, result } => {
            run_flow::finish(state, &engine, result)
        },
        ContainersIntent::ImageDefaultsLoaded {
            engine,
            reference,
            result,
        } => run_flow::defaults_loaded(state, &engine, &reference, result),
        ContainersIntent::FolderChecked { path, exists } => {
            run_flow::folder_checked(state, &path, exists)
        },
        ContainersIntent::RecheckFolders => run_flow::recheck_folders(state),
        ContainersIntent::FolderTypingPaused(path) => {
            run_flow::folder_typing_paused(state, &path)
        },
        ContainersIntent::FolderAnswerLate(path) => {
            run_flow::folder_answer_late(state, &path)
        },
    }
}

/// Make the inspector match the state after an intent: follow the logs of
/// the selected container only while the Logs tab is on screen in active
/// Containers mode, and load the inspect JSON once per selection when the Inspect
/// tab is on screen. A tab, selection or mode change stops the old logs, and so
/// does hiding the inspector; a selection change forgets the old JSON and its
/// open sections.
fn sync_inspector(state: &mut ContainersState) -> Task<ContainersEvent> {
    let log_target = state.wanted_log_target();
    state.inspector_mut().follow_logs(log_target);

    let selected = state.selected_inspect_target();
    state
        .inspector_mut()
        .forget_inspect_unless(selected.as_ref());
    refresh_inspect_tree(state);

    let Some((engine, target)) = state.wanted_inspect_target() else {
        return Task::none();
    };
    if !state.inspector_mut().start_inspect(target.clone()) {
        return Task::none();
    }

    effect(ContainersEffect::LoadInspect { engine, target })
}

/// Follow an Overview link: open a published port's page in the browser,
/// show the named item selected in its list of the open engine, or add the
/// `compose` tag of a Compose project (spec Decisions 20 and 21).
fn follow_link(
    state: &mut ContainersState,
    target: LinkTarget,
) -> Task<ContainersEvent> {
    match target {
        LinkTarget::Url(url) => effect(ContainersEffect::OpenUrl(url)),
        LinkTarget::Item(resource, id) => {
            if let Some(engine) =
                state.place().map(|place| place.engine().clone())
            {
                state.show_item(&engine, resource, id);
            }
            Task::none()
        },
        LinkTarget::ComposeProject(project) => {
            state.open_compose_project(&project);
            Task::none()
        },
    }
}

/// An engine event changed `resource`: reload it while Containers mode is
/// active. "Used by" of images, volumes and networks is read from the
/// containers shown now; a container reload brings its own list.
fn reload_changed_list(
    state: &ContainersState,
    address: &EngineAddress,
    resource: ResourceKind,
) -> Task<ContainersEvent> {
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

    effect(ContainersEffect::LoadList {
        info,
        resource,
        containers,
    })
}

/// Take a reloaded list. A new container list also reloads images, volumes
/// and networks with it, so their "used by" stays right; a failed one shows
/// its error and leaves the other lists as they are.
fn apply_reloaded_list(
    state: &mut ContainersState,
    address: &EngineAddress,
    list: LoadedList,
) -> Task<ContainersEvent> {
    let new_containers = match &list {
        LoadedList::Containers(Ok(containers)) => Some(containers.clone()),
        _ => None,
    };

    if !state.apply_list(address, list) {
        log::debug!("containers list of a gone engine dropped: {address}");
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
            effect(ContainersEffect::LoadList {
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
    state: &mut ContainersState,
    address: &EngineAddress,
) -> Task<ContainersEvent> {
    if active_engine_info(state, address).is_none()
        || state.search() == &Search::Searching
    {
        return Task::none();
    }

    state.start_search();
    effect(ContainersEffect::Discover)
}

/// The timer of the no-engine card ticked: search again behind the card, which
/// stays on screen. A tick that finds a search on its way is skipped, so a slow
/// search cannot stack.
fn search_behind_the_card(
    state: &mut ContainersState,
) -> Task<ContainersEvent> {
    if !state.is_active()
        || !state.shows_no_engine_card()
        || state.is_background_search_running()
    {
        return Task::none();
    }

    state.set_background_search_running(true);
    effect(ContainersEffect::DiscoverInBackground)
}

/// A search behind the no-engine card finished. It counts only while that card
/// is still on screen: during Check again the newer search decides, and with an
/// engine shown its "nothing here" is out of date. A result like the last one
/// changes nothing on the card; another one changes it, and an engine takes
/// over as after any search.
fn finish_search_behind_the_card(
    state: &mut ContainersState,
    discovery: Discovery,
) -> Task<ContainersEvent> {
    state.set_background_search_running(false);

    if !state.is_active() || !state.shows_no_engine_card() {
        log::debug!("containers search behind a card no longer shown dropped");
        return Task::none();
    }

    state.finish_search(discovery);
    load_every_engine(state)
}

/// The engine at `address` reconnected, so a start of the container whose logs
/// are open may have been missed: ask what became of it. Logs are open only
/// in active Containers mode.
fn check_open_logs(
    state: &ContainersState,
    address: &EngineAddress,
) -> Task<ContainersEvent> {
    let target = state
        .inspector()
        .logs()
        .map(|logs| logs.target())
        .filter(|target| target.is_on_engine(address));

    match target {
        Some(target) => effect(ContainersEffect::CheckLogRun(target.clone())),
        None => Task::none(),
    }
}

/// The engine at `address`, only while Containers mode is active.
fn active_engine_info(
    state: &ContainersState,
    address: &EngineAddress,
) -> Option<EngineInfo> {
    if !state.is_active() {
        return None;
    }

    state.engine(address).map(|engine| engine.info().clone())
}

/// Ask to load the lists of every engine, only while Containers mode is
/// active: a search that finishes after the user left starts nothing.
fn load_every_engine(state: &ContainersState) -> Task<ContainersEvent> {
    if !state.is_active() {
        return Task::none();
    }

    Task::batch(state.engines().iter().map(|engine| {
        effect(ContainersEffect::LoadLists(engine.info().clone()))
    }))
}

fn effect(effect: ContainersEffect) -> Task<ContainersEvent> {
    Task::done(ContainersEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use iced::widget::text_editor::Action;
    use otty_containers::{
        Container, ContainerRun, EngineInfo, EngineKind, Image, LogPosition,
        MAX_LOG_LINE_BYTES, Network, Volume, fixtures,
    };
    use otty_ui_kit::components::data_tree::DataRow;
    use otty_ui_kit::components::key_value_list::ValuePart;
    use otty_ui_kit::components::log_view::LogViewState;
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::i18n::{self, Key, Locale};
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::errors::ContainersError;
    use crate::widgets::containers::inspect_tree::InspectTree;
    use crate::widgets::containers::inspector::{
        InspectLoad, InspectTarget, InspectorTab, LogRead, LogTarget,
        MAX_LOG_LINES, MAX_LOG_TEXT_BYTES,
    };
    use crate::widgets::containers::inspector_tabs::{
        InspectContent, TabContent, inspector_tabs,
    };
    use crate::widgets::containers::model::{
        MainViewModel, main_area, panel_note,
    };
    use crate::widgets::containers::run_form::RunEdit;
    use crate::widgets::containers::sort::{SortOrder, TableSort};
    use crate::widgets::containers::state::{ListLoad, Search};
    use crate::widgets::containers::status::status_bar;
    use crate::widgets::containers::table_search::{
        COMPOSE_KEY, keys, keys_in,
    };
    use crate::widgets::containers::types::{
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
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let _ = reduce(
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
        let _ = reduce(state, ContainersIntent::Open(place));
    }

    fn selected_container(state: &ContainersState) -> Option<&str> {
        state.open_engine()?.containers().selected_id()
    }

    #[test]
    fn checking_again_starts_a_new_search() {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        assert!(matches!(state.search(), Search::Done(_)));

        let effects = effects(reduce(&mut state, ContainersIntent::CheckAgain));

        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![ContainersEffect::Discover]);
    }

    #[test]
    fn an_engine_started_meanwhile_is_found_by_checking_again() {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));

        assert_eq!(state.engines().len(), 1);
        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
    }

    #[test]
    fn copying_a_command_asks_for_the_clipboard() {
        let mut state = ContainersState::default();

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::CopyCommand(String::from(
                "sudo systemctl start docker",
            )),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::CopyToClipboard(String::from(
                "sudo systemctl start docker"
            ))]
        );
    }

    #[test]
    fn opening_a_link_asks_for_the_browser() {
        let mut state = ContainersState::default();

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::OpenLink(String::from("https://podman.io")),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::OpenUrl(String::from("https://podman.io"))]
        );
    }

    #[test]
    fn entering_containers_mode_searches_for_engines() {
        let mut state = ContainersState::default();

        let effects = effects(reduce(&mut state, ContainersIntent::Enter));

        assert!(state.is_active());
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![ContainersEffect::Discover]);
    }

    #[test]
    fn found_engines_load_their_lists_and_open_the_first_engines_containers() {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        ));

        assert_eq!(
            effects,
            vec![
                ContainersEffect::LoadLists(docker()),
                ContainersEffect::LoadLists(podman())
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
    fn no_engine_work_starts_outside_containers_mode() {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let after_search = effects(reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));
        let after_refresh =
            effects(reduce(&mut state, ContainersIntent::Refresh));

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
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
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

        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
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
            Some(engine.images().items()[0].row_key())
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
    fn opening_each_resource_shows_it_and_keeps_the_text_of_the_page_left() {
        let mut state = loaded_state();

        // Containers are open first, so they come back last.
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
            ResourceKind::Containers,
        ] {
            let left = state.place().map(Place::resource).expect("a page");
            type_text(&mut state, "otty");

            open(&mut state, resource);

            assert_eq!(state.place().map(Place::resource), Some(resource));
            assert_eq!(input_of(&state, left), "otty");
        }
    }

    /// Position in the shown table of the fixture container `name`.
    fn row_of(state: &ContainersState, name: &str) -> usize {
        let index = {
            let engine = state.open_engine().expect("docker engine");
            engine
                .containers()
                .items()
                .iter()
                .position(|c| c.name() == name)
                .expect("fixture container")
        };

        state.shown_position(index)
    }

    #[test]
    fn selecting_a_row_asks_to_reveal_the_inspector() {
        let mut state = loaded_state();

        let sent = effects(reduce(&mut state, ContainersIntent::SelectRow(1)));

        assert_eq!(sent, vec![ContainersEffect::RevealInspector]);
    }

    #[test]
    fn pressing_the_selected_row_again_still_asks_to_reveal_the_inspector() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let sent = effects(reduce(&mut state, ContainersIntent::SelectRow(1)));

        assert_eq!(sent, vec![ContainersEffect::RevealInspector]);
    }

    #[test]
    fn a_press_that_selects_no_row_asks_for_nothing() {
        let mut no_lists = ContainersState::default();
        let mut past_the_last_row = loaded_state();
        let before = selected_container(&past_the_last_row).map(str::to_string);

        let without_a_table =
            effects(reduce(&mut no_lists, ContainersIntent::SelectRow(0)));
        let without_that_row = effects(reduce(
            &mut past_the_last_row,
            ContainersIntent::SelectRow(usize::MAX),
        ));

        assert!(without_a_table.is_empty());
        assert!(without_that_row.is_empty());
        assert_eq!(
            selected_container(&past_the_last_row).map(str::to_string),
            before
        );
    }

    #[test]
    fn the_row_menu_button_the_right_click_and_a_port_link_do_not_reveal_the_inspector()
     {
        let mut state = loaded_state();
        let row = row_of(&state, "otty-fx-running");
        let position = iced::Point::ORIGIN;

        let button = effects(reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: row,
                position,
            },
        ));
        let right_click = effects(reduce(
            &mut state,
            ContainersIntent::OpenRowMenu {
                index: row,
                position,
            },
        ));
        let port_link = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink { row, link: 0 },
        ));

        assert!(button.is_empty());
        assert!(right_click.is_empty());
        assert!(matches!(
            port_link.as_slice(),
            [ContainersEffect::OpenUrl(_)]
        ));
    }

    #[test]
    fn a_port_link_opens_the_page_of_that_port_and_selects_nothing() {
        let mut state = loaded_state();
        let before = selected_container(&state).map(str::to_string);
        let row = row_of(&state, "otty-fx-running");

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink { row, link: 0 },
        ));

        assert_eq!(
            sent,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18080"
            ))]
        );
        assert_eq!(selected_container(&state).map(str::to_string), before);
    }

    #[test]
    fn a_port_link_is_looked_up_in_the_filtered_table() {
        let mut state = loaded_state();
        type_text(&mut state, "otty-fx-web");

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink { row: 0, link: 0 },
        ));

        assert_eq!(
            sent,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18081"
            ))]
        );
    }

    #[test]
    fn a_port_link_follows_its_row_when_the_ports_column_is_sorted() {
        const PORTS: usize = 4;
        let url_of = |state: &mut ContainersState, row: usize| {
            effects(reduce(
                state,
                ContainersIntent::OpenPortLink { row, link: 0 },
            ))
        };
        let opens = |port: u16| {
            vec![ContainersEffect::OpenUrl(format!(
                "http://localhost:{port}"
            ))]
        };
        let mut state = loaded_state();

        let _ = reduce(&mut state, ContainersIntent::SortBy(PORTS));

        let page = docker_page(ResourceKind::Containers);
        assert_eq!(state.sort(&page).column(), PORTS);
        assert_eq!(state.sort(&page).order(), SortOrder::Ascending);
        assert_eq!(
            shown_containers(&state),
            [
                "otty-fx-running",
                "otty-fx-web-1",
                "otty-fx-created",
                "otty-fx-exit0",
                "otty-fx-exit3",
                "otty-fx-paused",
            ]
        );
        assert_eq!(url_of(&mut state, 0), opens(18080));
        assert_eq!(url_of(&mut state, 1), opens(18081));
        assert!(url_of(&mut state, 2).is_empty());

        let _ = reduce(&mut state, ContainersIntent::SortBy(PORTS));

        assert_eq!(state.sort(&page).order(), SortOrder::Descending);
        assert_eq!(
            shown_containers(&state),
            [
                "otty-fx-created",
                "otty-fx-exit0",
                "otty-fx-exit3",
                "otty-fx-paused",
                "otty-fx-web-1",
                "otty-fx-running",
            ]
        );
        assert_eq!(url_of(&mut state, 4), opens(18081));
        assert_eq!(url_of(&mut state, 5), opens(18080));
        assert!(url_of(&mut state, 0).is_empty());
    }

    #[test]
    fn a_row_pressed_after_a_sort_by_ports_is_the_row_that_is_shown() {
        const PORTS: usize = 4;
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::SortBy(PORTS));

        let _ = reduce(&mut state, ContainersIntent::SelectRow(2));

        let selected = {
            let engine = state.open_engine().expect("docker engine");
            engine
                .containers()
                .selected(Container::id)
                .map(|container| container.name().to_string())
        };
        assert_eq!(selected.as_deref(), Some("otty-fx-created"));
    }

    #[test]
    fn the_sort_by_ports_is_kept_when_another_list_is_shown() {
        const PORTS: usize = 4;
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::SortBy(PORTS));

        open(&mut state, ResourceKind::Images);
        open(&mut state, ResourceKind::Containers);

        let sort = state.sort(&docker_page(ResourceKind::Containers));
        assert_eq!(sort.column(), PORTS);
        assert_eq!(state.sort(&docker_page(ResourceKind::Images)).column(), 0);
    }

    #[test]
    fn a_port_link_that_is_not_there_does_nothing() {
        let mut state = loaded_state();
        let row = row_of(&state, "otty-fx-running");

        for (row, link) in [(row, 1), (999, 0)] {
            let sent = effects(reduce(
                &mut state,
                ContainersIntent::OpenPortLink { row, link },
            ));
            assert!(sent.is_empty(), "{sent:?}");
        }

        open(&mut state, ResourceKind::Images);
        let sent = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink { row: 0, link: 0 },
        ));
        assert!(sent.is_empty(), "{sent:?}");

        let mut empty = ContainersState::default();
        let sent = effects(reduce(
            &mut empty,
            ContainersIntent::OpenPortLink { row: 0, link: 0 },
        ));
        assert!(sent.is_empty(), "{sent:?}");
    }

    #[test]
    fn a_compose_link_row_is_looked_up_in_the_filtered_table() {
        let mut state = loaded_state();
        type_text(&mut state, "web");

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Compose:otty-fx"]
        );
    }

    #[test]
    fn a_compose_link_on_a_row_without_a_project_changes_nothing() {
        let mut state = loaded_state();
        let index = {
            let engine = state.open_engine().expect("docker engine");
            engine
                .containers()
                .items()
                .iter()
                .position(|c| c.compose_project().is_none())
                .expect("a loose container")
        };
        let index = state.shown_position(index);

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(index));
        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(999));

        assert!(tags_of(&state, ResourceKind::Containers).is_empty());
    }

    #[test]
    fn a_header_press_sorts_the_open_table_and_a_second_press_reverses_it() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let _ = reduce(&mut state, ContainersIntent::SortBy(3));
        let first = state.sort(&docker_page(ResourceKind::Images));
        let _ = reduce(&mut state, ContainersIntent::SortBy(3));
        let second = state.sort(&docker_page(ResourceKind::Images));

        assert_eq!(first.column(), 3);
        assert_eq!(first.order(), SortOrder::Descending);
        assert_eq!(second.column(), 3);
        assert_eq!(second.order(), SortOrder::Ascending);
    }

    #[test]
    fn the_sort_of_a_table_is_kept_when_another_list_is_shown() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::SortBy(2));

        open(&mut state, ResourceKind::Volumes);
        let _ = reduce(&mut state, ContainersIntent::SortBy(1));
        open(&mut state, ResourceKind::Images);

        assert_eq!(state.sort(&docker_page(ResourceKind::Images)).column(), 2);
        assert_eq!(state.sort(&docker_page(ResourceKind::Volumes)).column(), 1);
        assert_eq!(
            state.sort(&docker_page(ResourceKind::Networks)).column(),
            0
        );
    }

    #[test]
    fn a_header_press_without_an_open_place_changes_nothing() {
        let mut state = ContainersState::default();

        let _ = reduce(&mut state, ContainersIntent::SortBy(2));

        assert_eq!(state.sort(&docker_page(ResourceKind::Images)).column(), 0);
    }

    #[test]
    fn the_selected_row_is_the_same_item_after_a_sort() {
        let mut state = loaded_state();
        let position = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        let id = |state: &ContainersState| {
            let engine = state.open_engine().expect("docker engine");
            engine
                .containers()
                .selected(Container::id)
                .map(|c| c.id().to_string())
        };
        let before = id(&state);

        let _ = reduce(&mut state, ContainersIntent::SortBy(0));

        assert_eq!(id(&state), before);
    }

    #[test]
    fn a_compose_link_outside_the_containers_list_changes_nothing() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        let place = state.place().expect("an open place");
        assert_eq!(place.resource(), ResourceKind::Images);
        assert!(tags_of(&state, ResourceKind::Containers).is_empty());
    }

    #[test]
    fn a_compose_link_without_an_open_engine_changes_nothing() {
        let mut state = ContainersState::default();

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        assert_eq!(state.place(), None);
    }

    fn follow(
        state: &mut ContainersState,
        target: LinkTarget,
    ) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::FollowLink(target)))
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

    /// The tree of the Inspect tab of the selected container, with `json` as
    /// the engine's answer.
    fn inspected_tree(state: &mut ContainersState, json: &str) -> InspectTree {
        let id = state.selected_id().expect("a selected row").to_string();
        let _ = reduce(
            state,
            ContainersIntent::SelectInspectorTab(InspectorTab::Inspect),
        );
        let _ = reduce(
            state,
            ContainersIntent::InspectLoaded {
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Containers,
                    &id,
                ),
                result: Ok(json.to_string()),
            },
        );

        let vm = inspector_tabs(state).expect("a selected container");
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
        assert_eq!(state.inspector().tab(), InspectorTab::Overview);
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

    #[test]
    fn an_image_link_shows_the_images_list_with_that_image_selected() {
        let mut state = loaded_state();
        let busybox = busybox_id(&state);
        let key = busybox_key(&state);
        open(&mut state, ResourceKind::Images);
        type_text(&mut state, "no such image");
        let _ = reduce(&mut state, ContainersIntent::SortBy(2));
        open(&mut state, ResourceKind::Containers);
        let _ = reduce(
            &mut state,
            ContainersIntent::SelectInspectorTab(InspectorTab::Inspect),
        );

        let effects = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Images, key.clone()),
        );

        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Images
            ))
        );
        assert_eq!(input_of(&state, ResourceKind::Images), "");
        assert_eq!(state.selected_id(), Some(key.as_str()));
        assert_eq!(state.inspector().tab(), InspectorTab::Overview);
        let (_, row) = state.selected_row().expect("the image is selected");
        assert_eq!(row.id(), busybox);
        // The image is shown in the table, sorted as the user left it.
        assert_eq!(state.sort(&docker_page(ResourceKind::Images)).column(), 2);
        let engine_index = state
            .open_engine()
            .expect("docker engine")
            .images()
            .items()
            .iter()
            .position(|image| image.id() == busybox)
            .expect("listed image");
        let position = state.shown_position(engine_index);
        assert_eq!(state.shown_row_id(position), Some(key.clone()));
        // A later sort keeps the same image selected.
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        assert_eq!(state.selected_id(), Some(key.as_str()));
    }

    #[test]
    fn a_container_link_shows_that_container_selected_past_the_tags() {
        let mut state = loaded_state();
        let running = {
            let engine = state.open_engine().expect("docker engine");
            engine
                .containers()
                .items()
                .iter()
                .find(|c| c.name() == "otty-fx-running")
                .map(|c| c.id().to_string())
                .expect("running container")
        };
        add_tag(&mut state, "compose:otty-fx");
        open(&mut state, ResourceKind::Volumes);

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Containers, running.clone()),
        );

        assert!(tags_of(&state, ResourceKind::Containers).is_empty());

        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Containers
            ))
        );
        assert_eq!(selected_container(&state), Some(running.as_str()));
    }

    #[test]
    fn a_volume_and_a_network_link_select_their_item() {
        let mut state = loaded_state();
        let network = {
            let engine = state.open_engine().expect("docker engine");
            engine
                .networks()
                .items()
                .iter()
                .find(|network| network.name() == "otty-fx-net")
                .map(|network| network.id().to_string())
                .expect("user network")
        };

        let _ = follow(
            &mut state,
            LinkTarget::Item(
                ResourceKind::Volumes,
                String::from("otty-fx-data"),
            ),
        );
        let volume = state.selected_id().map(str::to_string);
        let volume_place = state.place().map(Place::resource);
        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Networks, network.clone()),
        );

        assert_eq!(volume_place, Some(ResourceKind::Volumes));
        assert_eq!(volume.as_deref(), Some("otty-fx-data"));
        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Networks)
        );
        assert_eq!(state.selected_id(), Some(network.as_str()));
    }

    #[test]
    fn a_port_link_opens_its_page_and_keeps_the_list() {
        let mut state = loaded_state();
        let before = state.place().cloned();

        let effects = follow(
            &mut state,
            LinkTarget::Url(String::from("http://localhost:18081")),
        );

        assert_eq!(
            effects,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18081"
            ))]
        );
        assert_eq!(state.place().cloned(), before);
    }

    #[test]
    fn links_without_an_open_engine_change_nothing() {
        let mut state = ContainersState::default();

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Images, String::from("sha256:a7db")),
        );
        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        assert_eq!(state.place(), None);
    }

    #[test]
    fn a_row_press_in_a_compose_project_selects_that_project_container() {
        let mut state = loaded_state();
        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        let _ = reduce(&mut state, ContainersIntent::SelectRow(0));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-web-1"));
    }

    #[test]
    fn a_row_press_selects_that_row_of_the_filtered_table() {
        let mut state = loaded_state();
        type_text(&mut state, "exit");

        // The shown rows are exit0 and exit3 (same status, so by name).
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit3"));
    }

    #[test]
    fn a_row_press_on_each_resource_selects_its_row() {
        let mut state = loaded_state();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            let position = state.shown_position(1);
            let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        }

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.containers().selected(Container::id),
            engine.containers().items().get(1)
        );
        assert_eq!(
            engine.images().selected(Image::row_key),
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

        let _ = reduce(&mut state, ContainersIntent::SelectRow(99));

        assert_eq!(selected_container(&state).map(str::to_string), before);
    }

    #[test]
    fn a_reload_keeps_the_selection_while_the_row_is_there() {
        let mut state = loaded_state();
        let position = state.shown_position(2);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        let selected = selected_container(&state).map(str::to_string);
        let mut reloaded = lists(EngineKind::Docker);
        if let Ok(containers) = reloaded.containers.as_mut() {
            containers.remove(0);
        }

        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        assert_eq!(selected_container(&state).map(str::to_string), selected);
    }

    #[test]
    fn a_reload_without_the_selected_row_selects_the_first_row() {
        let mut state = loaded_state();
        let position = state.shown_position(2);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
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
            ContainersIntent::ListsLoaded {
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
        reloaded.images = Err(ContainersError::new("engine said no"));

        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
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
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );

        assert_eq!(state.engines().len(), 1);
        assert!(state.engine(podman().address()).is_none());
    }

    #[test]
    fn refresh_in_containers_mode_reloads_every_engine() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, ContainersIntent::Refresh));

        assert_eq!(
            effects,
            vec![
                ContainersEffect::LoadLists(docker()),
                ContainersEffect::LoadLists(podman())
            ]
        );
    }

    #[test]
    fn leaving_containers_mode_keeps_what_is_shown() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, ContainersIntent::Leave));

        assert!(effects.is_empty());
        assert!(!state.is_active());
        assert_eq!(state.engines().len(), 2);
        assert!(state.place().is_some());
    }

    // Inspector tabs.

    fn select_tab(
        state: &mut ContainersState,
        tab: InspectorTab,
    ) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::SelectInspectorTab(tab)))
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
        let _ = reduce(
            state,
            ContainersIntent::LogLines {
                target: log_target(index),
                lines,
                position: None,
            },
        );
    }

    fn inspect_of(resource: ResourceKind, id: &str) -> ContainersEffect {
        ContainersEffect::LoadInspect {
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

        let position = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }

    #[test]
    fn the_logs_stop_when_containers_mode_is_left_and_follow_again_on_return() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);

        let _ = reduce(&mut state, ContainersIntent::Leave);
        let after_leave = followed(&state);
        let _ = reduce(&mut state, ContainersIntent::Enter);

        assert_eq!(after_leave, None);
        assert_eq!(followed(&state), Some(container_id(0)));
    }

    fn hide_right_panel(
        state: &mut ContainersState,
        is_hidden: bool,
    ) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::RightPanelHidden(is_hidden)))
    }

    #[test]
    fn the_logs_stop_while_the_run_form_is_open_and_start_fresh_when_it_closes()
    {
        let mut state = logs_without_follow();

        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let while_open = followed(&state);
        let tab_while_open = state.inspector().tab();
        push_lines(&mut state, 0, &["late line of the old stream"]);
        let _ = reduce(&mut state, ContainersIntent::RunCancelled);

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
        let tab_while_hidden = state.inspector().tab();
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
        let tab_while_hidden = state.inspector().tab();
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
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let _ = hide_right_panel(&mut state, false);
        let form_still_open = followed(&state);
        let _ = hide_right_panel(&mut state, true);
        let _ = reduce(&mut state, ContainersIntent::RunCancelled);
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
        let _ = reduce(&mut state, ContainersIntent::Leave);
        let _ = hide_right_panel(&mut state, true);

        let _ = reduce(&mut state, ContainersIntent::Enter);
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
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let while_open = select_tab(&mut state, InspectorTab::Inspect);
        let on_close =
            effects(reduce(&mut state, ContainersIntent::RunCancelled));

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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_line_count(&state), 1);
    }

    #[test]
    fn a_reload_that_keeps_the_container_keeps_its_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["one", "two"]);

        let _ = reduce(
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

        let _ = reduce(
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
                ContainersIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                },
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
        let _ = reduce(state, ContainersIntent::LogFollowToggled(is_on));
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

        let _ = reduce(
            &mut state,
            ContainersIntent::LogAction(Action::Scroll { lines: -2 }),
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
            ContainersIntent::LogLines {
                target: log_target(0),
                lines,
                position: None,
            },
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
            let _ = reduce(
                state,
                ContainersIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                },
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

        let _ = reduce(
            state,
            ContainersIntent::LogAction(Action::Scroll { lines: -2 }),
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
            let _ = reduce(
                &mut state,
                ContainersIntent::LogLines {
                    target: log_target(0),
                    lines: batch.to_vec(),
                    position: None,
                },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::ContainerStarted {
                engine: docker().address().clone(),
                id: container_id(0),
            },
        );
        let target = state
            .inspector()
            .logs()
            .expect("followed logs")
            .target()
            .clone();

        for batch in longest_log_lines().chunks(50) {
            let _ = reduce(
                &mut state,
                ContainersIntent::LogLines {
                    target: target.clone(),
                    lines: batch.to_vec(),
                    position: None,
                },
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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        assert_eq!(followed(&state), Some(container_id(1)));
        assert!(log_view(&state).is_following());
    }

    #[test]
    fn follow_is_on_again_after_a_mode_change() {
        let mut state = logs_without_follow();

        let _ = reduce(&mut state, ContainersIntent::Leave);
        let _ = reduce(&mut state, ContainersIntent::Enter);

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
            ContainersIntent::LogsFailed {
                target: log_target(1),
                error: ContainersError::new("gone"),
            },
        );

        assert_eq!(log_line_count(&state), 0);
        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    /// A start event of the container at `index` of the fixture Docker engine.
    fn container_started(state: &mut ContainersState, index: usize) {
        let _ = reduce(
            state,
            ContainersIntent::ContainerStarted {
                engine: docker().address().clone(),
                id: container_id(index),
            },
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
            let _ = reduce(
                &mut state,
                ContainersIntent::LogLines {
                    target,
                    lines: vec![line.to_string()],
                    position: None,
                },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: old_stream,
                error: ContainersError::new("stream broke"),
            },
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    #[test]
    fn a_started_container_forgets_the_error_of_its_old_stream() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: log_target(0),
                error: ContainersError::new("stream broke"),
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogLines {
                target: stream,
                lines: vec![String::from("fresh")],
                position: None,
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogLines {
                target: stream,
                lines,
                position: None,
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogLines {
                target: stream.clone(),
                lines: vec![String::from("fresh")],
                position: None,
            },
        );

        let _ = reduce(
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

        let _ = reduce(
            &mut state,
            ContainersIntent::ContainerStarted {
                engine: podman().address().clone(),
                id: container_id(0),
            },
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

        let _ = reduce(
            &mut state,
            ContainersIntent::ListChanged {
                engine: docker().address().clone(),
                resource: ResourceKind::Containers,
            },
        );
        let _ = reduce(
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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
    }

    #[test]
    fn a_failed_log_stream_shows_the_engine_message_and_keeps_the_lines() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Logs);
        push_lines(&mut state, 0, &["starting"]);

        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: log_target(0),
                error: ContainersError::new("No such container"),
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

        let position = state.shown_position(1);
        let other =
            effects(reduce(&mut state, ContainersIntent::SelectRow(position)));
        let same =
            effects(reduce(&mut state, ContainersIntent::SelectRow(position)));

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
        let on_overview =
            effects(reduce(&mut state, ContainersIntent::SelectRow(position)));
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

        let _ = reduce(
            &mut state,
            ContainersIntent::InspectLoaded {
                target: container_inspect(0),
                result: Ok(json.clone()),
            },
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::InspectLoaded {
                target: container_inspect(1),
                result: Err(ContainersError::new("late")),
            },
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

        let _ = reduce(
            &mut state,
            ContainersIntent::InspectLoaded {
                target: container_inspect(0),
                result: Err(ContainersError::new("No such container")),
            },
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
        let _ = reduce(&mut state, ContainersIntent::Leave);

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
        let _ = reduce(
            state,
            ContainersIntent::InspectLoaded {
                target: container_inspect(index),
                result: Ok(json.to_string()),
            },
        );
    }

    fn toggle_section(state: &mut ContainersState, title: &str) {
        let _ = reduce(
            state,
            ContainersIntent::InspectSectionToggled(title.to_string()),
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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        load_inspect(&mut state, 1, TREE_JSON);

        assert_eq!(open_sections(&state), vec!["State"]);
    }

    #[test]
    fn a_selection_changed_on_another_tab_also_forgets_the_open_sections() {
        let mut state = state_with_inspect(TREE_JSON);
        toggle_section(&mut state, "Config");
        let _ = select_tab(&mut state, InspectorTab::Overview);

        let other = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(other));
        let first = state.shown_position(0);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(first));
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

        let _ =
            reduce(&mut state, ContainersIntent::InspectRawJsonToggled(true));
        let _ =
            reduce(&mut state, ContainersIntent::InspectShowEmptyToggled(true));
        let position = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        assert!(state.inspector().is_raw_json());
        assert!(state.inspector().shows_empty());

        let _ =
            reduce(&mut state, ContainersIntent::InspectRawJsonToggled(false));
        let _ = reduce(
            &mut state,
            ContainersIntent::InspectShowEmptyToggled(false),
        );

        assert!(!state.inspector().is_raw_json());
        assert!(!state.inspector().shows_empty());
    }

    #[test]
    fn copy_json_asks_for_the_clipboard_with_the_json_text() {
        let mut state = state_with_inspect("{\n  \"Id\": \"850b\"\n}");

        let effects =
            effects(reduce(&mut state, ContainersIntent::CopyInspectJson));

        assert_eq!(
            effects,
            vec![ContainersEffect::CopyToClipboard(String::from(
                "{\n  \"Id\": \"850b\"\n}"
            ))]
        );
    }

    #[test]
    fn copy_json_does_nothing_before_the_json_arrives() {
        let mut state = loaded_state();
        let _ = select_tab(&mut state, InspectorTab::Inspect);

        let effects =
            effects(reduce(&mut state, ContainersIntent::CopyInspectJson));

        assert!(effects.is_empty());
    }

    #[test]
    fn an_answer_that_is_not_an_object_keeps_its_text_and_opens_no_section() {
        let mut state = state_with_inspect("not json");

        let copied =
            effects(reduce(&mut state, ContainersIntent::CopyInspectJson));

        assert!(open_sections(&state).is_empty());
        assert_eq!(
            copied,
            vec![ContainersEffect::CopyToClipboard(String::from("not json"))]
        );
    }

    // Live updates from engine events (ticket 16).

    fn changed(resource: ResourceKind) -> ContainersIntent {
        ContainersIntent::ListChanged {
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
            vec![ContainersEffect::LoadList {
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
                vec![ContainersEffect::LoadList {
                    info: docker(),
                    resource,
                    containers: docker_containers(),
                }]
            );
        }
    }

    #[test]
    fn events_start_no_engine_work_outside_containers_mode() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::Leave);
        let docker_address = docker().address().clone();

        let intents = [
            changed(ResourceKind::Containers),
            changed(ResourceKind::Images),
            ContainersIntent::EngineReconnected(docker_address.clone()),
            ContainersIntent::EngineUnreachable(docker_address),
        ];

        for intent in intents {
            assert!(effects(reduce(&mut state, intent)).is_empty());
        }
        assert!(matches!(state.search(), Search::Done(_)));
    }

    #[test]
    fn events_of_an_unknown_engine_are_dropped() {
        let mut state = loaded_state();
        let unknown = otty_containers::EngineAddress::Tcp(String::from("h:1"));

        let intents = [
            ContainersIntent::ListChanged {
                engine: unknown.clone(),
                resource: ResourceKind::Containers,
            },
            ContainersIntent::EngineReconnected(unknown.clone()),
            ContainersIntent::EngineUnreachable(unknown),
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
            ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Ok(containers.clone())),
            },
        ));

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(engine.containers().items(), containers.as_slice());
        assert_eq!(
            effects,
            vec![
                ContainersEffect::LoadList {
                    info: docker(),
                    resource: ResourceKind::Images,
                    containers: containers.clone(),
                },
                ContainersEffect::LoadList {
                    info: docker(),
                    resource: ResourceKind::Volumes,
                    containers: containers.clone(),
                },
                ContainersEffect::LoadList {
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
            ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Err(ContainersError::new(
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
        let main = crate::widgets::containers::model::main_area(&state, 0);
        let crate::widgets::containers::model::MainViewModel::Table(table) =
            main
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
                ContainersIntent::ListLoaded {
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
        let gone = otty_containers::EngineAddress::Tcp(String::from("h:1"));

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ListLoaded {
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
            ContainersIntent::EngineReconnected(podman().address().clone()),
        ));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(podman())]);
    }

    #[test]
    fn an_unreachable_engine_starts_a_new_search() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        ));

        assert_eq!(effects, vec![ContainersEffect::Discover]);
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(state.engines().len(), 2);
    }

    #[test]
    fn an_unreachable_engine_during_a_search_starts_no_second_one() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        ));

        assert!(effects.is_empty());
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

        let _ = reduce(&mut state, reload(docker_containers()));
        let kept = (followed(&state), log_line_count(&state));
        let _ = reduce(&mut state, reload(docker_containers()[1..].to_vec()));

        assert_eq!(kept, (Some(container_id(0)), 1));
        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(log_line_count(&state), 0);
    }

    // The search behind the no-engine card (spec Decision 9).

    const DOCKER_CLI: &str = "/usr/bin/docker";
    const PODMAN_CLI: &str = "/usr/bin/podman";

    fn docker_stopped() -> otty_containers::Discovery {
        fixtures::discovery_with_cli(&[], Some(DOCKER_CLI), None)
    }

    fn podman_off() -> otty_containers::Discovery {
        fixtures::discovery_with_cli(&[], None, Some(PODMAN_CLI))
    }

    /// Containers mode active with the card of `discovery` on screen.
    fn card_of(discovery: otty_containers::Discovery) -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ =
            reduce(&mut state, ContainersIntent::DiscoveryFinished(discovery));
        assert!(state.shows_no_engine_card());

        state
    }

    fn tick(state: &mut ContainersState) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::EngineSearchTick))
    }

    fn background_result(
        state: &mut ContainersState,
        discovery: otty_containers::Discovery,
    ) -> Vec<ContainersEffect> {
        effects(reduce(
            state,
            ContainersIntent::BackgroundSearchFinished(discovery),
        ))
    }

    fn card(state: &ContainersState) -> MainViewModel<'_> {
        let vm = main_area(state, 0);
        assert!(matches!(vm, MainViewModel::NoEngine(_)), "{vm:?}");

        vm
    }

    /// The Docker socket refuses this user and Podman is off.
    fn docker_denied_podman_off() -> otty_containers::Discovery {
        fixtures::denied_discovery_with_cli(
            fixtures::DOCKER_SOCKET,
            otty_containers::CandidateSource::Docker,
            None,
            Some(PODMAN_CLI),
        )
    }

    /// A Podman socket refuses this user and Docker is off.
    fn podman_denied_docker_off() -> otty_containers::Discovery {
        fixtures::denied_discovery_with_cli(
            fixtures::PODMAN_SOCKET,
            otty_containers::CandidateSource::Podman,
            Some(DOCKER_CLI),
            None,
        )
    }

    #[test]
    fn a_denied_socket_with_the_other_off_keeps_searching_behind_the_card() {
        for discovery in
            [docker_denied_podman_off(), podman_denied_docker_off()]
        {
            let mut state = card_of(discovery.clone());
            let before = format!("{:?}", card(&state));

            let sent = tick(&mut state);
            assert_eq!(sent, vec![ContainersEffect::DiscoverInBackground]);

            let _ = background_result(&mut state, discovery);
            assert_eq!(format!("{:?}", card(&state)), before);

            let _ = tick(&mut state);
            let _ = background_result(
                &mut state,
                fixtures::discovery(&[EngineKind::Podman]),
            );
            assert!(!state.shows_no_engine_card());
        }
    }

    #[test]
    fn a_tick_searches_behind_the_card_and_the_card_stays() {
        let mut state = card_of(docker_stopped());
        let before = format!("{:?}", card(&state));
        let status = status_bar(&state);
        let note = panel_note(&state);

        let sent = tick(&mut state);

        assert_eq!(sent, vec![ContainersEffect::DiscoverInBackground]);
        assert!(matches!(state.search(), Search::Done(_)));
        assert_eq!(format!("{:?}", card(&state)), before);
        assert_eq!(status_bar(&state), status);
        assert_eq!(panel_note(&state), note);
    }

    #[test]
    fn a_tick_skips_while_its_search_is_on_its_way() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);

        assert!(tick(&mut state).is_empty());
        assert!(tick(&mut state).is_empty());
    }

    #[test]
    fn a_tick_searches_again_after_the_last_search_finished() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = background_result(&mut state, docker_stopped());

        assert_eq!(
            tick(&mut state),
            vec![ContainersEffect::DiscoverInBackground]
        );
    }

    #[test]
    fn a_tick_does_nothing_outside_containers_mode() {
        let mut state = card_of(docker_stopped());
        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert!(tick(&mut state).is_empty());
    }

    #[test]
    fn a_tick_does_nothing_while_engines_are_shown() {
        let mut state = loaded_state();

        assert!(tick(&mut state).is_empty());
    }

    #[test]
    fn a_tick_does_nothing_while_check_again_runs() {
        let mut state = card_of(docker_stopped());
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        assert!(tick(&mut state).is_empty());
    }

    #[test]
    fn the_same_answer_leaves_the_card_exactly_as_it_was() {
        let mut state = card_of(docker_stopped());
        let before = format!("{:?}", card(&state));
        let _ = tick(&mut state);

        let sent = background_result(&mut state, docker_stopped());

        assert!(sent.is_empty());
        assert_eq!(format!("{:?}", card(&state)), before);
    }

    #[test]
    fn another_answer_changes_the_card() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);

        let _ = background_result(&mut state, podman_off());

        let MainViewModel::NoEngine(vm) = card(&state) else {
            panic!("expected the card");
        };
        assert_eq!(vm.title, i18n::t(Key::ContainersPodmanOffTitle));
    }

    #[test]
    fn a_background_search_that_finds_an_engine_takes_the_normal_path() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);

        let sent = background_result(
            &mut state,
            fixtures::discovery(&[EngineKind::Docker]),
        );

        assert_eq!(sent, vec![ContainersEffect::LoadLists(docker())]);
        assert!(!state.shows_no_engine_card());
        assert_eq!(state.place().map(Place::engine), Some(docker().address()));
    }

    #[test]
    fn the_page_and_row_come_back_with_a_background_search_too() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);
        let row = select_item(&mut state, 1);
        find_engines(&mut state, &[]);
        let _ = tick(&mut state);

        let _ = background_result(
            &mut state,
            fixtures::discovery(&[EngineKind::Docker]),
        );
        lists_came(&mut state);

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Volumes)));
        assert_eq!(state.selected_id(), Some(row.as_str()));
    }

    /// Check again shows "searching" and waits for its own search, so what it
    /// shows next was looked up after the press.
    #[test]
    fn check_again_during_a_background_search_starts_its_own_search() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);

        let sent = effects(reduce(&mut state, ContainersIntent::CheckAgain));

        assert_eq!(sent, vec![ContainersEffect::Discover]);
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(main_area(&state, 0), MainViewModel::Searching);
    }

    #[test]
    fn a_background_result_does_not_end_the_search_of_check_again() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        let sent = background_result(&mut state, podman_off());

        assert!(sent.is_empty());
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(main_area(&state, 0), MainViewModel::Searching);

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );

        let MainViewModel::NoEngine(vm) = card(&state) else {
            panic!("expected the card");
        };
        assert_eq!(vm.title, i18n::t(Key::ContainersDockerStoppedTitle));
    }

    /// Check again found the engine while the older search was still on its
    /// way: the older "nothing here" must not wipe the table.
    #[test]
    fn an_old_background_result_does_not_undo_what_check_again_found() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let sent = background_result(&mut state, docker_stopped());

        assert!(sent.is_empty());
        assert_eq!(state.engines().len(), 1);
        assert_eq!(state.place().map(Place::engine), Some(docker().address()));
    }

    #[test]
    fn a_background_result_after_leaving_containers_mode_is_dropped() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let sent = background_result(
            &mut state,
            fixtures::discovery(&[EngineKind::Docker]),
        );

        assert!(sent.is_empty());
        assert!(state.engines().is_empty());
    }

    /// A slow search finished after the user came back: the next tick can
    /// search again, nothing stays stuck.
    #[test]
    fn a_result_after_a_round_trip_through_another_mode_frees_the_timer() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = reduce(&mut state, ContainersIntent::Leave);
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );
        assert!(tick(&mut state).is_empty());

        let _ = background_result(&mut state, docker_stopped());

        assert_eq!(
            tick(&mut state),
            vec![ContainersEffect::DiscoverInBackground]
        );
    }

    /// A press on Copy during a search or right after its result does what it
    /// always does.
    #[test]
    fn copy_works_while_a_background_search_runs() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let copy = ContainersIntent::CopyCommand(String::from(
            "sudo systemctl start docker",
        ));

        let during = effects(reduce(&mut state, copy.clone()));
        let _ = background_result(&mut state, docker_stopped());
        let after = effects(reduce(&mut state, copy));

        let copied = ContainersEffect::CopyToClipboard(String::from(
            "sudo systemctl start docker",
        ));
        assert_eq!(during, vec![copied.clone()]);
        assert_eq!(after, vec![copied]);
    }

    /// The form waits behind the card through the searches that find nothing
    /// and comes back with an engine, as it does after Check again.
    #[test]
    fn a_waiting_run_form_stays_through_background_searches() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = reduce(
            &mut state,
            ContainersIntent::RunEdited(RunEdit::Image(String::from("nginx"))),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );
        assert!(state.shows_no_engine_card());

        let _ = tick(&mut state);
        let _ = background_result(&mut state, docker_stopped());

        assert!(state.shows_no_engine_card());
        assert_eq!(
            state.run_form().map(|form| form.reference()),
            Some("nginx")
        );

        let _ = tick(&mut state);
        let _ = background_result(
            &mut state,
            fixtures::discovery(&[EngineKind::Docker]),
        );

        assert!(!state.shows_no_engine_card());
        assert_eq!(
            state.run_form().map(|form| form.reference()),
            Some("nginx")
        );
    }

    #[test]
    fn a_confirmation_stays_open_through_a_background_result() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::Prune);
        assert!(state.confirm().is_some());
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );

        let _ = tick(&mut state);
        let _ = background_result(&mut state, docker_stopped());

        assert!(state.confirm().is_some());
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
        let _ = reduce(
            state,
            ContainersIntent::LogRunRead {
                target,
                run: fixtures::container_run(started, true),
            },
        );
    }

    /// The followed stream sent `lines`, ending at `position`.
    fn lines_arrived(
        state: &mut ContainersState,
        lines: &[&str],
        position: Option<LogPosition>,
    ) {
        let target = followed_target(state);
        let _ = reduce(
            state,
            ContainersIntent::LogLines {
                target,
                lines: lines.iter().map(|line| line.to_string()).collect(),
                position,
            },
        );
    }

    /// The answer about the followed stream's container came.
    fn run_checked(
        state: &mut ContainersState,
        result: Result<ContainerRun, ContainersError>,
    ) {
        let target = followed_target(state);
        let _ =
            reduce(state, ContainersIntent::LogRunChecked { target, result });
    }

    fn reconnected(engine: &EngineInfo) -> ContainersIntent {
        ContainersIntent::EngineReconnected(engine.address().clone())
    }

    #[test]
    fn a_reconnect_asks_about_the_open_logs_and_reloads_the_lists() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let effects = effects(reduce(&mut state, reconnected(&docker())));

        assert_eq!(
            effects,
            vec![
                ContainersEffect::LoadLists(docker()),
                ContainersEffect::CheckLogRun(stream.clone()),
            ]
        );
        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    #[test]
    fn a_reconnect_of_another_engine_leaves_the_open_logs_alone() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);

        let effects = effects(reduce(&mut state, reconnected(&podman())));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(podman())]);
        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
    }

    #[test]
    fn a_reconnect_with_nothing_open_only_reloads_the_lists() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, reconnected(&docker())));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
        assert_eq!(followed(&state), None);
    }

    #[test]
    fn a_reconnect_of_the_engine_of_another_tab_starts_no_logs() {
        let mut state = open_logs_with_two_lines();
        let _ = select_tab(&mut state, InspectorTab::Overview);

        let effects = effects(reduce(&mut state, reconnected(&docker())));

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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogLines {
                target: old_stream.clone(),
                lines: vec![String::from("three")],
                position: Some(fixtures::log_position(RUN_STARTED + 6, 1)),
            },
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: old_stream,
                error: ContainersError::new("old stream dropped"),
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            },
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
            let _ = reduce(
                &mut state,
                ContainersIntent::LogLines {
                    target,
                    lines: batch.to_vec(),
                    position: Some(last),
                },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream.clone(),
                error: ContainersError::new(
                    "error reading a body from connection",
                ),
            },
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

        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream,
                error: ContainersError::new(
                    "error reading a body from connection",
                ),
            },
        );

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), None);
    }

    /// A failed answer says nothing about the container, so the message stays.
    #[test]
    fn a_failed_answer_keeps_the_error_of_the_broken_stream() {
        let mut state = open_logs_with_two_lines();
        let stream = followed_target(&state);
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream.clone(),
                error: ContainersError::new("connection closed"),
            },
        );

        run_checked(&mut state, Err(ContainersError::new("No such container")));

        let logs = state.inspector().logs().expect("followed logs");
        assert_eq!(logs.error(), Some("connection closed"));

        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("a later failure"),
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: stream,
                error: ContainersError::new("connection closed"),
            },
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
            let task = reduce(
                &mut state,
                ContainersIntent::LogRunChecked {
                    target: target.clone(),
                    result,
                },
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
        let _ = reduce(
            &mut state,
            ContainersIntent::LogsFailed {
                target: first.clone(),
                error: ContainersError::new("connection refused"),
            },
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

        let _ = reduce(
            &mut state,
            ContainersIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
            },
        );

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "fresh");
    }

    #[test]
    fn two_reconnects_in_a_row_give_one_new_stream() {
        let mut state = open_logs_with_two_lines();
        let asked_for = followed_target(&state);

        for _ in 0..2 {
            let _ = reduce(
                &mut state,
                ContainersIntent::LogRunChecked {
                    target: asked_for.clone(),
                    result: Ok(fixtures::container_run(RUN_STARTED, true)),
                },
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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        let stream = followed_target(&state);

        let _ = reduce(
            &mut state,
            ContainersIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED_AGAIN, true)),
            },
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
            let _ = reduce(&mut state, intent);

            let late = [
                ContainersIntent::LogRunChecked {
                    target: asked_for.clone(),
                    result: Ok(fixtures::container_run(RUN_STARTED, true)),
                },
                ContainersIntent::LogRunRead {
                    target: asked_for.clone(),
                    run: fixtures::container_run(RUN_STARTED, true),
                },
            ];
            for intent in late {
                let task = reduce(&mut state, intent);

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

        let _ = reduce(
            &mut state,
            ContainersIntent::LogRunRead {
                target: old_stream,
                run: fixtures::container_run(RUN_STARTED_AGAIN, true),
            },
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

        let _ = reduce(
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

        let _ = reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let effects = effects(reduce(&mut state, reconnected(&docker())));

        assert_eq!(followed_target(&state), stream);
        assert_eq!(log_view(&state).text(), "one\ntwo");
        assert!(effects.contains(&ContainersEffect::CheckLogRun(stream)));
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
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        let position = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        let _ = select_tab(&mut state, InspectorTab::Logs);
        run_read(&mut state, RUN_STARTED);
        lines_arrived(&mut state, &["one"], Some(seen()));
        let asked_for = followed_target(&state);
        assert_eq!(followed(&state), Some(container_id(1)));

        let _ = reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );

        assert!(state.engines().is_empty());
        assert_eq!(followed(&state), None);
        let task = reduce(
            &mut state,
            ContainersIntent::LogRunChecked {
                target: asked_for,
                result: Ok(fixtures::container_run(RUN_STARTED, true)),
            },
        );
        assert!(effects(task).is_empty());
        assert_eq!(followed(&state), None);

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        assert_eq!(followed(&state), None);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );

        assert_eq!(state.inspector().tab(), InspectorTab::Logs);
        assert_eq!(followed(&state), Some(container_id(1)));
        assert_eq!(followed_target(&state).read(), LogRead::LastLines);
        assert_eq!(log_line_count(&state), 0);
    }

    // The page and row come back with the only engine.

    /// Selects the row of the item the engine listed at `engine_index` in the
    /// open list and returns its id.
    fn select_item(state: &mut ContainersState, engine_index: usize) -> String {
        let position = state.shown_position(engine_index);
        let _ = reduce(state, ContainersIntent::SelectRow(position));

        state.selected_id().expect("a selected row").to_string()
    }

    /// The only engine goes away and is found again; its lists are not back
    /// yet.
    fn lose_and_find_again(state: &mut ContainersState) {
        find_engines(state, &[]);
        let _ = reduce(state, ContainersIntent::CheckAgain);
        find_engines(state, &[EngineKind::Docker]);
    }

    fn lists_came(state: &mut ContainersState) {
        let _ = reduce(
            state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
    }

    #[test]
    fn each_kind_of_page_and_its_row_come_back_with_the_only_engine() {
        for resource in ResourceKind::ALL {
            let mut state = loaded_state();
            open(&mut state, resource);
            let row = select_item(&mut state, 1);

            lose_and_find_again(&mut state);

            assert_eq!(
                state.place(),
                Some(&docker_page(resource)),
                "{resource:?}"
            );

            lists_came(&mut state);

            assert_eq!(state.selected_id(), Some(row.as_str()), "{resource:?}");
        }
    }

    /// Images are selected by their row key: one image can have several rows.
    #[test]
    fn an_image_row_comes_back_by_its_row_key() {
        let rows = fixtures::image_rows(
            "sha256:app",
            &["reg/app:1", "reg/app:2"],
            &[],
            &[],
        );
        let image_lists = || LoadedLists {
            images: Ok(rows.clone()),
            ..lists(EngineKind::Docker)
        };
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: image_lists(),
            },
        );
        let second = rows[1].row_key().to_string();
        let position = state.shown_position(1);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        assert_eq!(state.selected_id(), Some(second.as_str()));

        lose_and_find_again(&mut state);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: image_lists(),
            },
        );

        assert_eq!(state.selected_id(), Some(second.as_str()));
    }

    #[test]
    fn a_row_gone_meanwhile_falls_back_to_the_first_row() {
        let mut state = loaded_state();
        let _ = select_item(&mut state, 1);
        let without_it = {
            let mut containers = docker_containers();
            containers.remove(1);
            containers
        };
        let first = without_it[0].id().to_string();

        lose_and_find_again(&mut state);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(without_it),
                    ..lists(EngineKind::Docker)
                },
            },
        );

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Containers)));
        assert_eq!(state.selected_id(), Some(first.as_str()));
    }

    /// Events can reload the shown list before the four lists of the search
    /// come; the row comes back whichever comes first.
    #[test]
    fn the_row_comes_back_with_a_single_reloaded_list_too() {
        let mut state = loaded_state();
        let row = select_item(&mut state, 1);

        lose_and_find_again(&mut state);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListLoaded {
                engine: docker().address().clone(),
                list: LoadedList::Containers(Ok(docker_containers())),
            },
        );

        assert_eq!(state.selected_id(), Some(row.as_str()));
    }

    /// A failed list keeps the id, so the row is selected once it loads.
    #[test]
    fn the_row_comes_back_after_a_failed_first_load() {
        let mut state = loaded_state();
        let row = select_item(&mut state, 1);

        lose_and_find_again(&mut state);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists::failed(&ContainersError::new(
                    "engine busy",
                )),
            },
        );
        lists_came(&mut state);

        assert_eq!(state.selected_id(), Some(row.as_str()));
    }

    #[test]
    fn many_searches_that_find_nothing_keep_the_page_and_row_to_come_back_to() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);
        let row = select_item(&mut state, 1);

        for _ in 0..3 {
            find_engines(&mut state, &[]);
        }
        find_engines(&mut state, &[EngineKind::Docker]);
        lists_came(&mut state);

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Volumes)));
        assert_eq!(state.selected_id(), Some(row.as_str()));
    }

    /// The user moved on before the lists came: nothing jumps back.
    #[test]
    fn opening_another_page_before_the_lists_load_wins() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let _ = select_item(&mut state, 1);

        lose_and_find_again(&mut state);
        open(&mut state, ResourceKind::Networks);
        lists_came(&mut state);

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Networks)));
    }

    /// A link or a finished Run opened a page while no engine answered: that
    /// page wins over the one from before the outage.
    #[test]
    fn a_page_opened_during_the_outage_wins() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        find_engines(&mut state, &[]);

        open(&mut state, ResourceKind::Volumes);
        find_engines(&mut state, &[EngineKind::Docker]);

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Volumes)));
    }

    #[test]
    fn the_page_and_row_come_back_after_containers_mode_is_entered_again() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);
        let row = select_item(&mut state, 1);
        find_engines(&mut state, &[]);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let _ = reduce(&mut state, ContainersIntent::Enter);
        find_engines(&mut state, &[EngineKind::Docker]);
        lists_came(&mut state);

        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Volumes)));
        assert_eq!(state.selected_id(), Some(row.as_str()));
    }

    /// With two engines the view moved to the other one when its engine went
    /// away, so the engine's return must not pull it back.
    #[test]
    fn with_two_engines_a_returning_engine_does_not_pull_the_view_back() {
        let mut state = both_loaded();
        open(&mut state, ResourceKind::Images);
        let _ = select_item(&mut state, 1);
        let podman_containers = podman_page(ResourceKind::Containers);

        find_engines(&mut state, &[EngineKind::Podman]);
        assert_eq!(state.place(), Some(&podman_containers));

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);
        lists_came(&mut state);

        assert_eq!(state.place(), Some(&podman_containers));

        open(&mut state, ResourceKind::Images);

        let engine = state.open_engine().expect("docker engine");
        let first = engine.images().items()[0].row_key();
        assert_eq!(state.selected_id(), Some(first));
    }

    /// Another engine answered first: the view went there, and the engine it
    /// showed before must not pull it back later.
    #[test]
    fn the_page_is_forgotten_when_another_engine_is_found_first() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let podman_containers = podman_page(ResourceKind::Containers);
        find_engines(&mut state, &[]);

        find_engines(&mut state, &[EngineKind::Podman]);
        assert_eq!(state.place(), Some(&podman_containers));

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.place(), Some(&podman_containers));
    }

    /// Both engines were lost and come back together: the page shown last
    /// comes back, whichever engine is first in the search.
    #[test]
    fn the_shown_page_comes_back_when_both_engines_return_together() {
        let mut state = both_loaded();
        let page = podman_page(ResourceKind::Volumes);
        open_page(&mut state, page.clone());

        find_engines(&mut state, &[]);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.place(), Some(&page));
    }

    // Tag search.

    fn search_does(state: &mut ContainersState, action: TagSearchAction) {
        let _ = reduce(state, ContainersIntent::TagSearch(action));
    }

    fn type_text(state: &mut ContainersState, text: &str) {
        search_does(state, TagSearchAction::InputChanged(text.to_string()));
    }

    /// Adds the typed `key:text` as it is, as Enter does with no suggestion.
    fn add_tag(state: &mut ContainersState, text: &str) {
        type_text(state, text);
        search_does(state, TagSearchAction::Close);
        search_does(state, TagSearchAction::Apply);
    }

    /// The `resource` page of the fixture Docker engine.
    fn docker_page(resource: ResourceKind) -> Place {
        Place::new(docker().address().clone(), resource)
    }

    /// The `resource` page of the fixture Podman engine.
    fn podman_page(resource: ResourceKind) -> Place {
        Place::new(podman().address().clone(), resource)
    }

    /// The tags of `page` as the user reads them.
    fn tags_on(state: &ContainersState, page: &Place) -> Vec<String> {
        let keys = keys(page.resource());

        state
            .table_search(page)
            .tags()
            .iter()
            .map(|tag| tag.label(&keys))
            .collect()
    }

    /// The tags of the `resource` page of Docker.
    fn tags_of(state: &ContainersState, resource: ResourceKind) -> Vec<String> {
        tags_on(state, &docker_page(resource))
    }

    /// The text in the field of the `resource` page of Docker.
    fn input_of(state: &ContainersState, resource: ResourceKind) -> &str {
        state.table_search(&docker_page(resource)).input()
    }

    /// The names of the rows the containers table shows, top to bottom.
    fn shown_containers(state: &ContainersState) -> Vec<String> {
        let engine = state.open_engine().expect("docker engine");

        (0..)
            .map_while(|row| state.shown_row_id(row))
            .map(|id| {
                let container = engine.containers().find(&id, Container::id);

                container.expect("a listed container").name().to_string()
            })
            .collect()
    }

    #[test]
    fn typing_in_the_search_goes_to_the_search_of_the_open_table() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);

        type_text(&mut state, "dr");
        search_does(&mut state, TagSearchAction::Complete);

        assert_eq!(input_of(&state, ResourceKind::Volumes), "Driver:");
        assert_eq!(input_of(&state, ResourceKind::Containers), "");
    }

    #[test]
    fn a_search_action_with_nothing_open_changes_nothing() {
        let mut state = ContainersState::default();

        type_text(&mut state, "x");

        assert_eq!(input_of(&state, ResourceKind::Containers), "");
    }

    #[test]
    fn tags_per_table_survive_switching_lists_and_back() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");
        open(&mut state, ResourceKind::Images);
        add_tag(&mut state, "Repository:busybox");

        open(&mut state, ResourceKind::Containers);
        let containers = tags_of(&state, ResourceKind::Containers);
        let images = tags_of(&state, ResourceKind::Images);
        open(&mut state, ResourceKind::Images);

        assert_eq!(containers, vec!["Status:running"]);
        assert_eq!(images, vec!["Repository:busybox"]);
        assert_eq!(
            tags_of(&state, ResourceKind::Images),
            vec!["Repository:busybox"]
        );
    }

    #[test]
    fn leaving_the_mode_closes_the_suggestions_and_keeps_the_search() {
        let mut state = loaded_state();
        let page = docker_page(ResourceKind::Containers);
        add_tag(&mut state, "status:running");
        type_text(&mut state, "web");
        assert!(state.table_search(&page).is_open());

        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert!(!state.table_search(&page).is_open());
        assert_eq!(state.table_search(&page).input(), "web");
        assert_eq!(tags_on(&state, &page), vec!["Status:running"]);

        let _ = reduce(&mut state, ContainersIntent::Enter);

        assert!(!state.table_search(&page).is_open());
        assert_eq!(state.table_search(&page).input(), "web");
    }

    /// A list only opens on the page on screen, and going to another page
    /// closes it, so no list stays open behind the one left.
    #[test]
    fn leaving_the_mode_leaves_no_suggestions_open_on_any_page() {
        let mut state = loaded_state();
        let containers = docker_page(ResourceKind::Containers);
        let images = docker_page(ResourceKind::Images);
        type_text(&mut state, "web");
        open(&mut state, ResourceKind::Images);
        type_text(&mut state, "busy");
        assert!(state.table_search(&images).is_open());

        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert!(!state.table_search(&containers).is_open());
        assert!(!state.table_search(&images).is_open());
    }

    #[test]
    fn opening_another_list_keeps_the_text_and_closes_the_suggestions() {
        let mut state = loaded_state();
        let page = docker_page(ResourceKind::Containers);
        type_text(&mut state, "web");
        assert!(state.table_search(&page).is_open());

        open(&mut state, ResourceKind::Images);

        assert_eq!(state.table_search(&page).input(), "web");
        assert!(!state.table_search(&page).is_open());

        open(&mut state, ResourceKind::Containers);

        assert_eq!(state.table_search(&page).input(), "web");
        assert!(!state.table_search(&page).is_open());
    }

    /// The kept text filters as it did when the user left the page.
    #[test]
    fn coming_back_to_a_page_filters_by_its_kept_text() {
        let mut state = loaded_state();
        type_text(&mut state, "exit3");
        let filtered = shown_containers(&state);

        open(&mut state, ResourceKind::Images);
        open(&mut state, ResourceKind::Containers);

        assert_eq!(filtered.len(), 1);
        assert_eq!(shown_containers(&state), filtered);
    }

    #[test]
    fn a_link_keeps_the_text_of_the_page_it_leaves() {
        let mut state = loaded_state();
        let busybox = busybox_key(&state);
        type_text(&mut state, "web");

        let _ =
            follow(&mut state, LinkTarget::Item(ResourceKind::Images, busybox));

        assert_eq!(input_of(&state, ResourceKind::Containers), "web");
    }

    #[test]
    fn losing_the_only_engine_closes_the_suggestions_and_keeps_the_text() {
        let mut state = loaded_state();
        let page = docker_page(ResourceKind::Containers);
        type_text(&mut state, "web");
        assert!(state.table_search(&page).is_open());

        find_engines(&mut state, &[]);

        assert_eq!(state.table_search(&page).input(), "web");
        assert!(!state.table_search(&page).is_open());

        find_engines(&mut state, &[EngineKind::Docker]);

        assert_eq!(state.place(), Some(&page));
        assert_eq!(state.table_search(&page).input(), "web");
        assert!(!state.table_search(&page).is_open());
    }

    #[test]
    fn a_place_moved_from_a_lost_engine_closes_the_suggestions_it_left() {
        let mut state = both_loaded();
        let page = docker_page(ResourceKind::Containers);
        type_text(&mut state, "web");

        find_engines(&mut state, &[EngineKind::Podman]);

        assert_eq!(state.table_search(&page).input(), "web");
        assert!(!state.table_search(&page).is_open());
    }

    #[test]
    fn a_search_that_keeps_the_place_leaves_the_suggestions_open() {
        let mut state = loaded_state();
        let page = docker_page(ResourceKind::Containers);
        type_text(&mut state, "web");

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert!(state.table_search(&page).is_open());
    }

    #[test]
    fn opening_the_shown_list_again_keeps_the_text() {
        let mut state = loaded_state();
        type_text(&mut state, "web");

        open(&mut state, ResourceKind::Containers);

        assert_eq!(input_of(&state, ResourceKind::Containers), "web");
    }

    #[test]
    fn clear_all_removes_the_tags_of_this_table_only() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        add_tag(&mut state, "Repository:busybox");
        open(&mut state, ResourceKind::Containers);
        add_tag(&mut state, "Status:running");
        add_tag(&mut state, "Image:busybox");

        search_does(&mut state, TagSearchAction::ClearTags);

        assert!(tags_of(&state, ResourceKind::Containers).is_empty());
        assert_eq!(
            tags_of(&state, ResourceKind::Images),
            vec!["Repository:busybox"]
        );
    }

    #[test]
    fn a_single_cross_removes_that_tag() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");
        add_tag(&mut state, "Image:busybox");

        search_does(&mut state, TagSearchAction::RemoveTag(0));

        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Image:busybox"]
        );
    }

    #[test]
    fn a_tag_key_shows_the_header_of_the_language_it_is_read_in() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");
        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        let tags = state
            .table_search(&docker_page(ResourceKind::Containers))
            .tags();
        let in_english = keys_in(Locale::En, ResourceKind::Containers);
        let in_russian = keys_in(Locale::Ru, ResourceKind::Containers);
        let in_chinese = keys_in(Locale::ZhCn, ResourceKind::Containers);

        assert_eq!(tags[0].key(), "status");
        assert_eq!(tags[0].label(&in_english), "Status:running");
        assert_eq!(tags[0].label(&in_russian), "Состояние:running");
        assert_eq!(tags[0].label(&in_chinese), "状态:running");
    }

    #[test]
    fn a_compose_link_replaces_the_compose_tag_by_its_internal_name() {
        let mut state = loaded_state();
        add_tag(&mut state, "Compose:other");

        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        let tags = state
            .table_search(&docker_page(ResourceKind::Containers))
            .tags();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), COMPOSE_KEY);
        assert_eq!(tags[0].value(), "otty-fx");
    }

    #[test]
    fn a_compose_link_adds_a_tag_and_keeps_the_list() {
        let mut state = loaded_state();
        let row = row_of(&state, "otty-fx-web-1");

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(row));

        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Compose:otty-fx"]
        );
        assert_eq!(shown_containers(&state), vec!["otty-fx-web-1"]);
    }

    #[test]
    fn a_compose_link_replaces_another_compose_tag_and_keeps_other_keys() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");
        add_tag(&mut state, "Compose:other");
        add_tag(&mut state, "Compose:another");

        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Status:running", "Compose:otty-fx"]
        );
    }

    #[test]
    fn a_compose_link_clears_the_text_so_the_project_shows() {
        let mut state = loaded_state();
        type_text(&mut state, "web");

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        assert_eq!(input_of(&state, ResourceKind::Containers), "");
    }

    #[test]
    fn a_compose_link_row_is_looked_up_among_the_tagged_rows() {
        let mut state = loaded_state();
        add_tag(&mut state, "Name:web");

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(0));

        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Name:web", "Compose:otty-fx"]
        );
    }

    #[test]
    fn a_compose_link_in_another_list_opens_the_containers_with_the_tag() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);

        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Containers)
        );
        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Compose:otty-fx"]
        );
        assert!(tags_of(&state, ResourceKind::Volumes).is_empty());
    }

    #[test]
    fn a_link_to_a_hidden_item_removes_the_tags_of_the_target_table() {
        let mut state = loaded_state();
        let busybox = busybox_key(&state);
        open(&mut state, ResourceKind::Images);
        add_tag(&mut state, "Repository:otty-fx-img");
        open(&mut state, ResourceKind::Containers);
        add_tag(&mut state, "Status:running");

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Images, busybox.clone()),
        );

        assert!(tags_of(&state, ResourceKind::Images).is_empty());
        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Status:running"]
        );
        assert_eq!(state.selected_id(), Some(busybox.as_str()));
    }

    #[test]
    fn a_link_to_a_visible_item_keeps_the_tags_of_the_target_table() {
        let mut state = loaded_state();
        let busybox = busybox_key(&state);
        open(&mut state, ResourceKind::Images);
        add_tag(&mut state, "Repository:busybox");
        open(&mut state, ResourceKind::Containers);

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Images, busybox.clone()),
        );

        assert_eq!(
            tags_of(&state, ResourceKind::Images),
            vec!["Repository:busybox"]
        );
        assert_eq!(state.selected_id(), Some(busybox.as_str()));
    }

    #[test]
    fn a_link_to_each_kind_of_item_keeps_tags_that_show_it() {
        let mut state = loaded_state();
        let resources = fixtures::resources(EngineKind::Docker);
        let cases = [
            (
                ResourceKind::Volumes,
                "Name:otty-fx-data",
                resources.volumes()[1].name().to_string(),
            ),
            (
                ResourceKind::Networks,
                "Name:bridge",
                resources.networks()[0].id().to_string(),
            ),
            (
                ResourceKind::Containers,
                "Name:otty-fx-web-1",
                resources.containers()[0].id().to_string(),
            ),
        ];

        for (resource, tag, id) in cases {
            open(&mut state, resource);
            add_tag(&mut state, tag);
            open(&mut state, ResourceKind::Images);

            let _ = follow(&mut state, LinkTarget::Item(resource, id));

            assert_eq!(tags_of(&state, resource), vec![tag], "{resource:?}");
        }
    }

    #[test]
    fn a_link_forgets_the_text_of_the_target_table_even_on_that_table() {
        let mut state = loaded_state();
        let first = state.selected_id().expect("a selected row").to_string();
        type_text(&mut state, "no such name");

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Containers, first),
        );

        assert_eq!(input_of(&state, ResourceKind::Containers), "");
    }

    #[test]
    fn a_link_to_an_item_not_loaded_yet_removes_the_tags() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Containers, String::from("new")),
        );

        assert!(tags_of(&state, ResourceKind::Containers).is_empty());
    }

    #[test]
    fn a_row_press_selects_that_row_among_the_tagged_rows() {
        let mut state = loaded_state();
        add_tag(&mut state, "Status:running");

        let names = shown_containers(&state);
        assert_eq!(names.len(), 2, "two running containers: {names:?}");

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some(names[1].as_str()));
    }

    #[test]
    fn the_row_menu_opens_for_that_row_among_the_tagged_rows() {
        let mut state = loaded_state();
        add_tag(&mut state, "Name:exit3");

        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        );

        let exit3 = docker_containers()
            .into_iter()
            .find(|container| container.name() == "otty-fx-exit3")
            .expect("fixture container");
        assert_eq!(state.row_menu_target(), Some(exit3.id()));
    }

    #[test]
    fn a_port_link_is_looked_up_among_the_tagged_rows() {
        let mut state = loaded_state();
        add_tag(&mut state, "Ports:18081");

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink { row: 0, link: 0 },
        ));

        assert_eq!(
            sent,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18081"
            ))]
        );
    }

    // Engines open and close like folders.

    fn press_root(
        state: &mut ContainersState,
        engine: &EngineInfo,
    ) -> Vec<ContainersEffect> {
        effects(reduce(
            state,
            ContainersIntent::ToggleEngine(engine.address().clone()),
        ))
    }

    fn find_engines(state: &mut ContainersState, kinds: &[EngineKind]) {
        let _ = reduce(
            state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
    }

    #[test]
    fn a_press_on_an_engine_root_leaves_the_right_side_as_it_is() {
        let mut state = loaded_state();
        add_tag(&mut state, "compose:otty-fx");
        type_text(&mut state, "web");
        let _ = reduce(&mut state, ContainersIntent::SelectRow(0));
        let _ = select_tab(&mut state, InspectorTab::Inspect);
        state.set_notice(String::from("the engine said no"));
        let place = state.place().cloned();
        let selected = selected_container(&state).map(str::to_string);

        for engine in [podman(), docker(), docker(), podman()] {
            let effects = press_root(&mut state, &engine);

            assert_eq!(effects, Vec::new(), "{engine:?}");
            assert_eq!(state.place().cloned(), place);
            assert_eq!(
                selected_container(&state).map(str::to_string),
                selected
            );
            assert_eq!(input_of(&state, ResourceKind::Containers), "web");
            assert_eq!(
                tags_of(&state, ResourceKind::Containers),
                vec!["Compose:otty-fx"]
            );
            assert_eq!(state.inspector().tab(), InspectorTab::Inspect);
            assert_eq!(state.notice(), Some("the engine said no"));
        }
    }

    #[test]
    fn a_press_on_a_root_tells_which_engines_are_closed() {
        let mut state = loaded_state();

        let _ = press_root(&mut state, &podman());
        let podman_closed = state.is_engine_closed(podman().address());
        let docker_closed = state.is_engine_closed(docker().address());
        let _ = press_root(&mut state, &podman());

        assert!(podman_closed);
        assert!(!docker_closed);
        assert!(!state.is_engine_closed(podman().address()));
    }

    #[test]
    fn closed_engines_stay_closed_across_a_mode_switch_and_a_refresh() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());
        let _ = press_root(&mut state, &podman());

        let _ = reduce(&mut state, ContainersIntent::Leave);
        let _ = reduce(&mut state, ContainersIntent::Enter);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);
        let _ = reduce(&mut state, ContainersIntent::Refresh);

        assert!(state.is_engine_closed(docker().address()));
        assert!(state.is_engine_closed(podman().address()));
        assert_eq!(state.place().map(Place::engine), Some(docker().address()));
    }

    #[test]
    fn a_closed_engine_that_goes_away_and_comes_back_is_still_closed() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &podman());

        find_engines(&mut state, &[EngineKind::Docker]);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert!(state.is_engine_closed(podman().address()));
    }

    #[test]
    fn a_search_that_moves_the_place_opens_the_engine_it_moves_to() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &podman());

        find_engines(&mut state, &[EngineKind::Podman]);

        assert_eq!(state.place().map(Place::engine), Some(podman().address()));
        assert!(!state.is_engine_closed(podman().address()));
    }

    #[test]
    fn a_closed_engine_found_again_after_no_engine_stays_closed_and_is_shown() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());

        find_engines(&mut state, &[]);
        find_engines(&mut state, &[EngineKind::Docker]);

        assert_eq!(state.place().map(Place::engine), Some(docker().address()));
        assert!(state.is_engine_closed(docker().address()));
    }

    #[test]
    fn a_closed_engine_found_by_check_again_stays_closed_beside_a_new_one() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());
        find_engines(&mut state, &[]);

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.place().map(Place::engine), Some(docker().address()));
        assert!(state.is_engine_closed(docker().address()));
        assert!(!state.is_engine_closed(podman().address()));
    }

    #[test]
    fn a_place_that_moves_opens_its_new_engine_and_the_lost_one_stays_closed() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());
        let _ = press_root(&mut state, &podman());

        find_engines(&mut state, &[EngineKind::Podman]);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.place().map(Place::engine), Some(podman().address()));
        assert!(!state.is_engine_closed(podman().address()));
        assert!(state.is_engine_closed(docker().address()));
    }

    #[test]
    fn a_lost_engine_takes_its_row_menu_and_notice_with_it() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        );
        state.set_notice(String::from("the engine said no"));

        find_engines(&mut state, &[]);

        assert_eq!(state.row_menu(), None);
        assert_eq!(state.notice(), None);
    }

    #[test]
    fn a_search_that_keeps_the_place_keeps_the_row_menu_and_the_notice() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        );
        state.set_notice(String::from("the engine said no"));

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.row_menu(), Some(iced::Point::ORIGIN));
        assert_eq!(state.notice(), Some("the engine said no"));
    }

    #[test]
    fn an_item_link_opens_the_closed_engine_of_the_list_it_shows() {
        let mut state = loaded_state();
        let busybox = busybox_key(&state);
        let _ = press_root(&mut state, &docker());
        let _ = press_root(&mut state, &podman());

        let _ =
            follow(&mut state, LinkTarget::Item(ResourceKind::Images, busybox));

        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Images)
        );
        assert!(!state.is_engine_closed(docker().address()));
        assert!(state.is_engine_closed(podman().address()));
    }

    #[test]
    fn a_compose_link_opens_the_closed_engine_on_the_list_already_shown() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());

        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        assert!(!state.is_engine_closed(docker().address()));
    }

    #[test]
    fn a_compose_link_of_a_table_row_opens_the_closed_engine() {
        let mut state = loaded_state();
        let row = row_of(&state, "otty-fx-web-1");
        let _ = press_root(&mut state, &docker());

        let _ = reduce(&mut state, ContainersIntent::OpenComposeProject(row));

        assert!(!state.is_engine_closed(docker().address()));
    }

    #[test]
    fn a_port_link_goes_to_no_list_and_leaves_the_engine_closed() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &docker());

        let _ = follow(
            &mut state,
            LinkTarget::Url(String::from("http://localhost:18081")),
        );

        assert!(state.is_engine_closed(docker().address()));
    }

    #[test]
    fn an_event_of_the_engine_not_shown_reloads_that_engines_list() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );
        let podman_containers = fixtures::resources(EngineKind::Podman)
            .containers()
            .to_vec();

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ListChanged {
                engine: podman().address().clone(),
                resource: ResourceKind::Images,
            },
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::LoadList {
                info: podman(),
                resource: ResourceKind::Images,
                containers: podman_containers,
            }]
        );
    }

    #[test]
    fn a_reloaded_list_of_the_engine_not_shown_updates_that_engine_only() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );
        let docker_images = state
            .engine(docker().address())
            .expect("docker engine")
            .images()
            .items()
            .len();

        let _ = reduce(
            &mut state,
            ContainersIntent::ListLoaded {
                engine: podman().address().clone(),
                list: LoadedList::Images(Ok(Vec::new())),
            },
        );

        let engine = state.engine(podman().address()).expect("podman engine");
        assert!(engine.images().items().is_empty());
        assert_eq!(
            state
                .engine(docker().address())
                .expect("docker engine")
                .images()
                .items()
                .len(),
            docker_images
        );
    }

    #[test]
    fn a_closed_engine_keeps_its_lists_live() {
        let mut state = loaded_state();
        let _ = press_root(&mut state, &podman());

        let refreshed = effects(reduce(&mut state, ContainersIntent::Refresh));
        let changed = effects(reduce(
            &mut state,
            ContainersIntent::ListChanged {
                engine: podman().address().clone(),
                resource: ResourceKind::Containers,
            },
        ));

        assert_eq!(
            refreshed,
            vec![
                ContainersEffect::LoadLists(docker()),
                ContainersEffect::LoadLists(podman())
            ]
        );
        assert_eq!(
            changed,
            vec![ContainersEffect::LoadList {
                info: podman(),
                resource: ResourceKind::Containers,
                containers: Vec::new(),
            }]
        );
    }

    // One search and sort per page: a page is one list of one engine.

    /// Both fixture engines found with their lists loaded, Docker's containers
    /// open.
    fn both_loaded() -> ContainersState {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );

        state
    }

    fn open_page(state: &mut ContainersState, page: Place) {
        let _ = reduce(state, ContainersIntent::Open(page));
    }

    /// Podman's containers open, Docker's containers narrowed to one row and
    /// sorted by name. Only a position read on Podman's page finds the row
    /// `PODMAN_WEB` below.
    fn podman_open_with_docker_filtered() -> ContainersState {
        let mut state = both_loaded();
        add_tag(&mut state, "Name:exit3");
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        open_page(&mut state, podman_page(ResourceKind::Containers));

        state
    }

    /// Podman's containers run first, so `otty-fx-web-1` is the second row.
    const PODMAN_WEB: usize = 1;

    /// The id of the fixture Podman container `otty-fx-web-1`.
    fn podman_web_id() -> String {
        fixtures::resources(EngineKind::Podman)
            .containers()
            .iter()
            .find(|container| container.name() == "otty-fx-web-1")
            .map(|container| container.id().to_string())
            .expect("fixture container")
    }

    /// The id of the selected container of Podman.
    fn podman_selected(state: &ContainersState) -> Option<&str> {
        state.engine(podman().address())?.containers().selected_id()
    }

    #[test]
    fn a_tag_stays_on_its_page_when_the_other_engine_shows_the_same_list() {
        let mut state = both_loaded();
        let docker_page = docker_page(ResourceKind::Containers);
        let podman_page = podman_page(ResourceKind::Containers);
        add_tag(&mut state, "Compose:no-such-project");

        open_page(&mut state, podman_page.clone());

        assert!(tags_on(&state, &podman_page).is_empty());
        assert_eq!(shown_containers(&state).len(), 6);

        open_page(&mut state, docker_page.clone());

        assert_eq!(
            tags_on(&state, &docker_page),
            vec!["Compose:no-such-project"]
        );
        assert!(shown_containers(&state).is_empty());
    }

    #[test]
    fn the_typed_text_stays_on_its_page() {
        let mut state = both_loaded();
        let podman = podman_page(ResourceKind::Containers);

        type_text(&mut state, "web");

        assert_eq!(input_of(&state, ResourceKind::Containers), "web");
        assert_eq!(state.table_search(&podman).input(), "");
    }

    #[test]
    fn opening_another_page_keeps_the_text_of_every_page() {
        let mut state = both_loaded();
        type_text(&mut state, "web");
        find_engines(&mut state, &[EngineKind::Podman]);
        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);
        let podman_containers = podman_page(ResourceKind::Containers);
        type_text(&mut state, "exit");

        open_page(&mut state, podman_page(ResourceKind::Images));

        assert_eq!(state.table_search(&podman_containers).input(), "exit");
        assert_eq!(input_of(&state, ResourceKind::Containers), "web");
    }

    #[test]
    fn a_sort_press_changes_the_open_page_only() {
        let mut state = both_loaded();
        let docker_containers = docker_page(ResourceKind::Containers);
        let podman_containers = podman_page(ResourceKind::Containers);
        let untouched = state.sort(&podman_containers);

        let _ = reduce(&mut state, ContainersIntent::SortBy(0));

        assert_eq!(state.sort(&docker_containers).column(), 0);
        assert_eq!(state.sort(&podman_containers), untouched);

        open_page(&mut state, podman_containers.clone());
        let _ = reduce(&mut state, ContainersIntent::SortBy(5));
        open_page(&mut state, docker_containers.clone());

        assert_eq!(state.sort(&docker_containers).column(), 0);
        assert_eq!(state.sort(&podman_containers).column(), 5);
    }

    #[test]
    fn a_page_keeps_its_tags_and_sort_after_a_tour_of_every_other_page() {
        let mut state = both_loaded();
        let page = docker_page(ResourceKind::Images);
        open_page(&mut state, page.clone());
        add_tag(&mut state, "Repository:busybox");
        let _ = reduce(&mut state, ContainersIntent::SortBy(3));
        let sort = state.sort(&page);

        for resource in ResourceKind::ALL {
            open_page(&mut state, docker_page(resource));
            open_page(&mut state, podman_page(resource));
        }
        open_page(&mut state, page.clone());

        assert_eq!(tags_on(&state, &page), vec!["Repository:busybox"]);
        assert_eq!(state.sort(&page), sort);
    }

    #[test]
    fn a_row_press_is_read_through_the_search_of_the_open_page() {
        let mut state = podman_open_with_docker_filtered();

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::SelectRow(PODMAN_WEB),
        ));

        assert_eq!(sent, vec![ContainersEffect::RevealInspector]);
        assert_eq!(podman_selected(&state), Some(podman_web_id().as_str()));
    }

    #[test]
    fn a_right_press_is_read_through_the_search_of_the_open_page() {
        let mut state = podman_open_with_docker_filtered();

        let _ = reduce(
            &mut state,
            ContainersIntent::OpenRowMenu {
                index: PODMAN_WEB,
                position: iced::Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu_target(), Some(podman_web_id().as_str()));
        assert_eq!(podman_selected(&state), Some(podman_web_id().as_str()));
    }

    #[test]
    fn the_row_menu_button_is_read_through_the_search_of_the_open_page() {
        let mut state = podman_open_with_docker_filtered();

        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: PODMAN_WEB,
                position: iced::Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu_target(), Some(podman_web_id().as_str()));
    }

    #[test]
    fn a_port_link_is_read_through_the_search_of_the_open_page() {
        let mut state = podman_open_with_docker_filtered();

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::OpenPortLink {
                row: PODMAN_WEB,
                link: 0,
            },
        ));

        assert_eq!(
            sent,
            vec![ContainersEffect::OpenUrl(String::from(
                "http://localhost:18081"
            ))]
        );
    }

    #[test]
    fn a_compose_link_of_a_row_is_read_through_the_search_of_the_open_page() {
        let mut state = podman_open_with_docker_filtered();

        let _ = reduce(
            &mut state,
            ContainersIntent::OpenComposeProject(PODMAN_WEB),
        );

        assert_eq!(
            tags_on(&state, &podman_page(ResourceKind::Containers)),
            vec!["Compose:otty-fx"]
        );
        assert_eq!(
            tags_of(&state, ResourceKind::Containers),
            vec!["Name:exit3"]
        );
    }

    #[test]
    fn a_compose_link_adds_its_tag_on_the_containers_page_of_its_engine_only() {
        let mut state = both_loaded();
        open_page(&mut state, podman_page(ResourceKind::Images));

        let _ = follow(
            &mut state,
            LinkTarget::ComposeProject(String::from("otty-fx")),
        );

        let podman_containers = podman_page(ResourceKind::Containers);
        assert_eq!(state.place(), Some(&podman_containers));
        assert_eq!(
            tags_on(&state, &podman_containers),
            vec!["Compose:otty-fx"]
        );
        assert!(tags_of(&state, ResourceKind::Containers).is_empty());
    }

    #[test]
    fn a_link_to_a_hidden_item_removes_the_tags_of_its_page_only() {
        let mut state = both_loaded();
        open_page(&mut state, docker_page(ResourceKind::Images));
        add_tag(&mut state, "Repository:otty-fx-img");
        let podman_images = podman_page(ResourceKind::Images);
        open_page(&mut state, podman_images.clone());
        add_tag(&mut state, "Repository:no-such-image");
        let busybox = fixtures::resources(EngineKind::Podman)
            .images()
            .iter()
            .find(|image| {
                image.reference() == Some("docker.io/library/busybox:1.37")
            })
            .map(|image| image.row_key().to_string())
            .expect("fixture image");

        let _ = follow(
            &mut state,
            LinkTarget::Item(ResourceKind::Images, busybox.clone()),
        );

        assert!(tags_on(&state, &podman_images).is_empty());
        assert_eq!(
            tags_of(&state, ResourceKind::Images),
            vec!["Repository:otty-fx-img"]
        );
        assert_eq!(state.selected_id(), Some(busybox.as_str()));
    }

    #[test]
    fn a_page_keeps_its_search_and_sort_when_its_only_engine_comes_back() {
        let mut state = loaded_state();
        let page = docker_page(ResourceKind::Containers);
        add_tag(&mut state, "Status:running");
        type_text(&mut state, "web");
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        let sort = state.sort(&page);

        find_engines(&mut state, &[]);
        find_engines(&mut state, &[EngineKind::Docker]);

        assert_eq!(state.place(), Some(&page));
        assert_eq!(tags_on(&state, &page), vec!["Status:running"]);
        assert_eq!(state.table_search(&page).input(), "web");
        assert_eq!(state.sort(&page), sort);
    }

    #[test]
    fn a_page_left_because_its_engine_went_away_keeps_its_search_and_sort() {
        let mut state = both_loaded();
        let docker_containers = docker_page(ResourceKind::Containers);
        let podman_containers = podman_page(ResourceKind::Containers);
        add_tag(&mut state, "Status:running");
        type_text(&mut state, "web");
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        let sort = state.sort(&docker_containers);

        find_engines(&mut state, &[EngineKind::Podman]);

        assert_eq!(state.place(), Some(&podman_containers));
        assert!(tags_on(&state, &podman_containers).is_empty());
        assert_eq!(state.table_search(&podman_containers).input(), "");
        assert_eq!(
            state.sort(&podman_containers),
            TableSort::default_for(ResourceKind::Containers)
        );

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);
        open_page(&mut state, docker_containers.clone());

        assert_eq!(tags_on(&state, &docker_containers), vec!["Status:running"]);
        assert_eq!(state.table_search(&docker_containers).input(), "web");
        assert_eq!(state.sort(&docker_containers), sort);
    }

    #[test]
    fn what_a_page_sent_is_taken_while_that_page_is_open() {
        let mut state = both_loaded();

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::FromPage {
                page: docker_page(ResourceKind::Containers),
                intent: Box::new(ContainersIntent::SelectRow(1)),
            },
        ));

        assert_eq!(sent, vec![ContainersEffect::RevealInspector]);
    }

    #[test]
    fn what_the_page_left_sent_changes_nothing_on_the_open_page() {
        let left = docker_page(ResourceKind::Containers);
        let open = podman_page(ResourceKind::Containers);
        let late_intents = [
            ContainersIntent::SelectRow(1),
            ContainersIntent::OpenRowMenu {
                index: 1,
                position: iced::Point::ORIGIN,
            },
            ContainersIntent::RowMenuPressed {
                index: 1,
                position: iced::Point::ORIGIN,
            },
            ContainersIntent::OpenPortLink { row: 1, link: 0 },
            ContainersIntent::OpenComposeProject(1),
            ContainersIntent::SortBy(0),
            ContainersIntent::TagSearch(TagSearchAction::InputChanged(
                String::from("web"),
            )),
            ContainersIntent::Prune,
        ];

        for intent in late_intents {
            let mut state = both_loaded();
            open_page(&mut state, open.clone());
            let selected = podman_selected(&state).map(str::to_string);
            let sort = state.sort(&open);

            let sent = effects(reduce(
                &mut state,
                ContainersIntent::FromPage {
                    page: left.clone(),
                    intent: Box::new(intent.clone()),
                },
            ));

            assert!(sent.is_empty(), "{intent:?}");
            assert_eq!(
                podman_selected(&state).map(str::to_string),
                selected,
                "{intent:?}"
            );
            assert_eq!(state.row_menu_target(), None, "{intent:?}");
            assert!(state.confirm().is_none(), "{intent:?}");
            assert!(tags_on(&state, &open).is_empty(), "{intent:?}");
            assert_eq!(state.table_search(&open).input(), "", "{intent:?}");
            assert_eq!(state.sort(&open), sort, "{intent:?}");
            assert_eq!(state.place(), Some(&open), "{intent:?}");
        }
    }

    #[test]
    fn what_a_page_sent_is_dropped_once_a_lost_engine_moved_the_place() {
        let mut state = both_loaded();
        let page = docker_page(ResourceKind::Containers);
        find_engines(&mut state, &[EngineKind::Podman]);
        let selected = podman_selected(&state).map(str::to_string);

        let sent = effects(reduce(
            &mut state,
            ContainersIntent::FromPage {
                page,
                intent: Box::new(ContainersIntent::SelectRow(1)),
            },
        ));

        assert!(sent.is_empty());
        assert_eq!(podman_selected(&state).map(str::to_string), selected);
    }

    #[test]
    fn an_age_tag_counts_ages_with_the_kept_clock() {
        const NOW: i64 = 1_793_557_960;
        const MUCH_LATER: i64 = NOW + 3_000_000;
        let mut state = loaded_state();
        let resources = fixtures::resources(EngineKind::Docker);
        let created = resources.containers()[0].created();
        let age = crate::widgets::containers::format::age_text(NOW, created);
        state.set_age_clock(NOW);
        add_tag(&mut state, &format!("Age:{age}"));

        let at_now = shown_containers(&state).len();
        state.set_age_clock(MUCH_LATER);
        let much_later = shown_containers(&state).len();

        assert!(at_now > 0);
        assert_eq!(much_later, 0);
    }
}
