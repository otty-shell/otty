#![forbid(clippy::disallowed_methods)]

use iced::{Point, Task};
use otty_containers::{Discovery, EngineAddress, EngineInfo};

use super::child::actions::ActionsOutcome;
use super::child::actions::event::{ActionsEffect, ActionsIntent};
use super::child::create::CreateOutcome;
use super::child::create::event::{CreateEffect, CreateIntent};
use super::child::inspector::InspectorOutcome;
use super::child::inspector::event::{InspectorEffect, InspectorIntent};
use super::child::run::RunOutcome;
use super::child::run::event::{RunEffect, RunIntent};
use super::child::{actions, create, inspector, run};
use super::event::{ContainersEffect, ContainersEvent, ContainersIntent};
use super::links::LinkTarget;
use super::state::{ContainersState, Search};
use super::types::{LoadedList, ResourceKind};

/// Reduce a Containers intent into state changes and effects. Engine work
/// (search, list loads, inspect) is only asked for while Containers mode is
/// active (spec Decision 14); the reducer itself does no I/O. After every
/// intent the inspector follows the selection, the open tab and whether it
/// is on screen: see [`InspectorIntent::FollowSelection`].
pub(super) fn reduce(
    state: &mut ContainersState,
    intent: ContainersIntent,
) -> Task<ContainersEvent> {
    let task = reduce_intent(state, intent);

    let inspector = reduce_inspector(state, InspectorIntent::FollowSelection);
    Task::batch([task, inspector])
}

/// Hand `intent` to the create dialogs and act on what they report in the
/// same update.
fn reduce_create(
    state: &mut ContainersState,
    intent: CreateIntent,
) -> Task<ContainersEvent> {
    let (effects, outcome) = create::reduce(state.create_mut(), intent);
    let after = outcome
        .map_or_else(Task::none, |outcome| on_create_outcome(state, outcome));

    Task::batch([effects.map(wrap_create_effect), after])
}

/// Hand `intent` to the Run container form and act on what it reports in
/// the same update.
fn reduce_run(
    state: &mut ContainersState,
    intent: RunIntent,
) -> Task<ContainersEvent> {
    let (run, ctx) = state.run_and_ctx();
    let (effects, outcome) = run::reduce(run, intent, &ctx);
    let after = outcome
        .map_or_else(Task::none, |outcome| on_run_outcome(state, outcome));

    Task::batch([effects.map(wrap_run_effect), after])
}

/// Hand `intent` to the actions and act on what they report in the same
/// update.
fn reduce_actions(
    state: &mut ContainersState,
    intent: ActionsIntent,
) -> Task<ContainersEvent> {
    let (actions, ctx) = state.actions_and_ctx();
    let (effects, outcome) = actions::reduce(actions, intent, &ctx);
    let after = outcome
        .map_or_else(Task::none, |outcome| on_actions_outcome(state, outcome));

    Task::batch([effects.map(wrap_actions_effect), after])
}

/// Hand `intent` to the inspector and act on what it reports in the same
/// update.
fn reduce_inspector(
    state: &mut ContainersState,
    intent: InspectorIntent,
) -> Task<ContainersEvent> {
    let (inspector, ctx) = state.inspector_and_ctx();
    let (effects, outcome) = inspector::reduce(inspector, intent, &ctx);
    let after = outcome.map_or_else(Task::none, |outcome| {
        on_inspector_outcome(state, outcome)
    });

    Task::batch([effects.map(wrap_inspector_effect), after])
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
            let folders = reduce_run(state, RunIntent::RecheckFolders);
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
            take_search_result(state, discovery)
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
            let closed = reduce_run(state, RunIntent::Close);
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
        ContainersIntent::SelectInspectorTab(tab) => {
            state.select_inspector_tab(tab);
            Task::none()
        },
        ContainersIntent::ListChanged { engine, resource } => {
            reload_changed_list(state, &engine, resource)
        },
        ContainersIntent::EngineReconnected(engine) => {
            let lists = reload_engine(state, &engine);
            let logs = reduce_inspector(
                state,
                InspectorIntent::EngineReconnected(engine),
            );

            Task::batch([lists, logs])
        },
        ContainersIntent::EngineUnreachable(engine) => {
            search_after_lost_engine(state, &engine)
        },
        ContainersIntent::ListLoaded { engine, list } => {
            apply_reloaded_list(state, &engine, list)
        },
        ContainersIntent::OpenRowMenu { index, position } => {
            let _ = state.select_shown_row(index);
            open_row_menu(state, index, position)
        },
        ContainersIntent::RowMenuPressed { index, position } => {
            open_row_menu(state, index, position)
        },
        ContainersIntent::OpenCreate => open_create(state),
        ContainersIntent::Create(intent) => reduce_create(state, intent),
        ContainersIntent::Run(intent) => reduce_run(state, intent),
        ContainersIntent::Actions(intent) => reduce_actions(state, intent),
        ContainersIntent::Inspector(intent) => reduce_inspector(state, intent),
    }
}

/// Follow a link of the inspector's Overview or Inspect tree: open a
/// published port's page in the browser, show the named item selected in its
/// list of the open engine, or add the `compose` tag of a Compose project
/// (spec Decisions 20 and 21).
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

    take_search_result(state, discovery)
}

/// Take a finished engine search and load the lists of every engine. A place
/// lost with its engine takes the row menu of its list with it.
fn take_search_result(
    state: &mut ContainersState,
    discovery: Discovery,
) -> Task<ContainersEvent> {
    let is_place_lost = state.finish_search(discovery);

    let closed = if is_place_lost {
        reduce_actions(state, ActionsIntent::CloseRowMenu)
    } else {
        Task::none()
    };

    Task::batch([closed, load_every_engine(state)])
}

/// Reload every list of the engine at `address`, only while Containers mode
/// is active and the engine is still known.
fn reload_engine(
    state: &ContainersState,
    address: &EngineAddress,
) -> Task<ContainersEvent> {
    match active_engine_info(state, address) {
        Some(info) => effect(ContainersEffect::LoadLists(info)),
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

/// Open the create dialog of the open list: Pull image, Create volume or
/// Create network; the containers list opens the Run container form.
fn open_create(state: &mut ContainersState) -> Task<ContainersEvent> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return Task::none();
    };
    let info = engine.info().clone();

    let open = match place.resource() {
        ResourceKind::Images => CreateIntent::OpenPull {
            engine: info,
            image: String::new(),
            tag: String::new(),
        },
        ResourceKind::Volumes => CreateIntent::OpenVolume(info),
        ResourceKind::Networks => CreateIntent::OpenNetwork(info),
        ResourceKind::Containers => {
            let open = RunIntent::Open {
                engine: info,
                image: String::new(),
            };
            return reduce_run(state, open);
        },
    };

    reduce_create(state, open)
}

/// Act on what the create dialogs report: a sent volume or network runs as
/// an action, a finished pull reloads its engine's lists and a failed one
/// tells the user.
fn on_create_outcome(
    state: &mut ContainersState,
    outcome: CreateOutcome,
) -> Task<ContainersEvent> {
    match outcome {
        CreateOutcome::ActionRequested(request) => {
            reduce_actions(state, ActionsIntent::Start(request))
        },
        CreateOutcome::Pulled { engine } => reload_engine(state, &engine),
        CreateOutcome::Failed(text) => effect(ContainersEffect::Failed(text)),
    }
}

/// Act on what the Run container form reports: a started container shows
/// selected in the Containers table of its engine, whose lists reload; a
/// failed run or pull tells the user while the form stays as typed.
fn on_run_outcome(
    state: &mut ContainersState,
    outcome: RunOutcome,
) -> Task<ContainersEvent> {
    match outcome {
        RunOutcome::Started { engine, id } => {
            state.show_item(&engine, ResourceKind::Containers, id);
            reload_engine(state, &engine)
        },
        RunOutcome::Failed(text) => effect(ContainersEffect::Failed(text)),
    }
}

/// Act on what the actions report: a finished action reloads its engine's
/// lists and a failed one tells the user; Run and Pull again on an image
/// row open the Run form and the pull dialog.
fn on_actions_outcome(
    state: &mut ContainersState,
    outcome: ActionsOutcome,
) -> Task<ContainersEvent> {
    match outcome {
        ActionsOutcome::Done { engine } => reload_engine(state, &engine),
        ActionsOutcome::Failed(text) => effect(ContainersEffect::Failed(text)),
        ActionsOutcome::RunRequested { engine, image } => {
            reduce_run(state, RunIntent::Open { engine, image })
        },
        ActionsOutcome::PullRequested { engine, image, tag } => {
            reduce_create(state, CreateIntent::OpenPull { engine, image, tag })
        },
    }
}

/// Act on what the inspector reports: a pressed link is followed.
fn on_inspector_outcome(
    state: &mut ContainersState,
    outcome: InspectorOutcome,
) -> Task<ContainersEvent> {
    match outcome {
        InspectorOutcome::FollowLink(target) => follow_link(state, target),
    }
}

/// Open the menu of the row at `index` of the shown table at `position`.
/// Nothing opens past the last row.
fn open_row_menu(
    state: &mut ContainersState,
    index: usize,
    position: Point,
) -> Task<ContainersEvent> {
    let Some(target) = state.shown_row_id(index) else {
        return Task::none();
    };

    reduce_actions(state, ActionsIntent::OpenRowMenu { target, position })
}

fn effect(effect: ContainersEffect) -> Task<ContainersEvent> {
    Task::done(ContainersEvent::Effect(effect))
}

fn wrap_create_effect(effect: CreateEffect) -> ContainersEvent {
    ContainersEvent::Effect(ContainersEffect::Create(effect))
}

fn wrap_run_effect(effect: RunEffect) -> ContainersEvent {
    ContainersEvent::Effect(ContainersEffect::Run(effect))
}

fn wrap_actions_effect(effect: ActionsEffect) -> ContainersEvent {
    ContainersEvent::Effect(ContainersEffect::Actions(effect))
}

fn wrap_inspector_effect(effect: InspectorEffect) -> ContainersEvent {
    ContainersEvent::Effect(ContainersEffect::Inspector(effect))
}

#[cfg(test)]
mod tests {
    use otty_containers::{
        Container, EngineInfo, EngineKind, Image, Network, Volume, fixtures,
    };
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::i18n::{self, Key, Locale};
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::child::actions::PruneViewModel;
    use crate::widgets::containers::child::create::FormDialogViewModel;
    use crate::widgets::containers::child::run::types::RunEdit;
    use crate::widgets::containers::model::{
        EngineTreeViewModel, MainAreaViewModel, StatusBarViewModel,
    };
    use crate::widgets::containers::sort::{SortOrder, TableSort};
    use crate::widgets::containers::state::{ListLoad, Search};
    use crate::widgets::containers::table_search::{
        COMPOSE_KEY, keys, keys_in,
    };
    use crate::widgets::containers::types::{
        ActionRequest, ContainersError, EngineAction, InspectorTab, LoadedList,
        LoadedLists, Place, ResourceKind,
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

    /// Press the link to `target` in the inspector.
    fn follow(
        state: &mut ContainersState,
        target: LinkTarget,
    ) -> Vec<ContainersEffect> {
        let pressed = InspectorIntent::LinkPressed(target);

        effects(reduce(state, ContainersIntent::Inspector(pressed)))
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

    #[test]
    fn the_follow_link_outcome_of_the_inspector_shows_its_item_selected() {
        let mut state = loaded_state();
        let key = busybox_key(&state);
        let pressed = InspectorIntent::LinkPressed(LinkTarget::Item(
            ResourceKind::Images,
            key.clone(),
        ));

        let task = reduce(&mut state, ContainersIntent::Inspector(pressed));

        assert!(effects(task).is_empty());
        assert_eq!(state.place(), Some(&docker_page(ResourceKind::Images)));
        assert_eq!(state.selected_id(), Some(key.as_str()));
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
        assert_eq!(state.inspector_tab(), InspectorTab::Overview);
        let row = state.selected_row().expect("the image is selected");
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
        let main = MainAreaViewModel::from_state(&state, 0);
        let MainAreaViewModel::Table(table) = main else {
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

    fn card(state: &ContainersState) -> MainAreaViewModel<'_> {
        let vm = MainAreaViewModel::from_state(state, 0);
        assert!(matches!(vm, MainAreaViewModel::NoEngine(_)), "{vm:?}");

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
        let status = StatusBarViewModel::from_state(&state);
        let note = EngineTreeViewModel::from_state(&state).note;

        let sent = tick(&mut state);

        assert_eq!(sent, vec![ContainersEffect::DiscoverInBackground]);
        assert!(matches!(state.search(), Search::Done(_)));
        assert_eq!(format!("{:?}", card(&state)), before);
        assert_eq!(StatusBarViewModel::from_state(&state), status);
        assert_eq!(EngineTreeViewModel::from_state(&state).note, note);
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

        let MainAreaViewModel::NoEngine(vm) = card(&state) else {
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
        assert_eq!(
            MainAreaViewModel::from_state(&state, 0),
            MainAreaViewModel::Searching
        );
    }

    #[test]
    fn a_background_result_does_not_end_the_search_of_check_again() {
        let mut state = card_of(docker_stopped());
        let _ = tick(&mut state);
        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        let sent = background_result(&mut state, podman_off());

        assert!(sent.is_empty());
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(
            MainAreaViewModel::from_state(&state, 0),
            MainAreaViewModel::Searching
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );

        let MainAreaViewModel::NoEngine(vm) = card(&state) else {
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
            ContainersIntent::Run(RunIntent::Edited(RunEdit::Image(
                String::from("nginx"),
            ))),
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
            state.run().form().map(|form| form.reference()),
            Some("nginx")
        );

        let _ = tick(&mut state);
        let _ = background_result(
            &mut state,
            fixtures::discovery(&[EngineKind::Docker]),
        );

        assert!(!state.shows_no_engine_card());
        assert_eq!(
            state.run().form().map(|form| form.reference()),
            Some("nginx")
        );
    }

    #[test]
    fn a_confirmation_stays_open_through_a_background_result() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);
        let _ =
            reduce(&mut state, ContainersIntent::Actions(ActionsIntent::Prune));
        assert!(state.actions().confirm().is_some());
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(docker_stopped()),
        );

        let _ = tick(&mut state);
        let _ = background_result(&mut state, docker_stopped());

        assert!(state.actions().confirm().is_some());
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
        assert_eq!(state.actions().row_menu_target(), Some(exit3.id()));
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
            assert_eq!(state.inspector_tab(), InspectorTab::Inspect);
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
    fn a_lost_engine_takes_its_row_menu_with_it() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        );

        find_engines(&mut state, &[]);

        assert_eq!(state.actions().row_menu(), None);
    }

    #[test]
    fn a_search_that_keeps_the_place_keeps_the_row_menu() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        );

        find_engines(&mut state, &[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(state.actions().row_menu(), Some(iced::Point::ORIGIN));
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

        assert_eq!(
            state.actions().row_menu_target(),
            Some(podman_web_id().as_str())
        );
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

        assert_eq!(
            state.actions().row_menu_target(),
            Some(podman_web_id().as_str())
        );
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
            ContainersIntent::Actions(ActionsIntent::Prune),
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
            assert_eq!(state.actions().row_menu_target(), None, "{intent:?}");
            assert!(state.actions().confirm().is_none(), "{intent:?}");
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

    #[test]
    fn the_create_button_opens_the_dialog_of_each_list() {
        let cases = [
            (ResourceKind::Images, Key::ContainersPullImage),
            (ResourceKind::Volumes, Key::ContainersCreateVolume),
            (ResourceKind::Networks, Key::ContainersCreateNetwork),
        ];

        for (resource, title) in cases {
            let mut state = loaded_state();
            open(&mut state, resource);

            let effects =
                effects(reduce(&mut state, ContainersIntent::OpenCreate));

            assert!(effects.is_empty());
            assert_eq!(
                FormDialogViewModel::from_state(state.create())
                    .map(|vm| vm.title),
                Some(i18n::t(title))
            );
        }
    }

    #[test]
    fn the_containers_create_button_opens_the_run_form() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Containers);

        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        assert!(!state.create().is_open());
        assert!(state.run().is_open());
    }

    #[test]
    fn a_create_request_marks_its_list_busy_and_runs_the_call() {
        let mut state = loaded_state();
        let request = ActionRequest::new(
            docker(),
            EngineAction::CreateVolume {
                name: String::from("otty-fx-vol"),
                driver: String::from("local"),
            },
            String::from("otty-fx-vol"),
        );
        let address = docker().address().clone();
        let was_busy =
            state
                .actions()
                .is_busy(&address, ResourceKind::Volumes, None);

        let effects = effects(on_create_outcome(
            &mut state,
            CreateOutcome::ActionRequested(request.clone()),
        ));

        assert!(!was_busy);
        assert!(
            state
                .actions()
                .is_busy(&address, ResourceKind::Volumes, None)
        );
        assert_eq!(
            effects,
            vec![ContainersEffect::Actions(ActionsEffect::RunAction(request))]
        );
    }

    #[test]
    fn a_pulled_image_reloads_its_engine_only_while_containers_mode_is_active()
    {
        let mut state = loaded_state();
        let pulled = || CreateOutcome::Pulled {
            engine: docker().address().clone(),
        };

        let active = effects(on_create_outcome(&mut state, pulled()));
        let _ = reduce(&mut state, ContainersIntent::Leave);
        let inactive = effects(on_create_outcome(&mut state, pulled()));

        assert_eq!(active, vec![ContainersEffect::LoadLists(docker())]);
        assert!(inactive.is_empty());
    }

    #[test]
    fn a_failed_create_tells_the_user() {
        let mut state = loaded_state();

        let effects = effects(on_create_outcome(
            &mut state,
            CreateOutcome::Failed(String::from("Could not pull nginx:latest")),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::Failed(String::from(
                "Could not pull nginx:latest"
            ))]
        );
    }

    /// Owner, Q1: a failed run is a toast and a list entry; the form stays
    /// open as typed, without a notice of its own.
    #[test]
    fn a_failed_run_tells_the_user_and_keeps_the_place() {
        let mut state = loaded_state();
        let place = state.place().cloned();

        let effects = effects(on_run_outcome(
            &mut state,
            RunOutcome::Failed(String::from("Could not run web: busy")),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::Failed(String::from(
                "Could not run web: busy"
            ))]
        );
        assert_eq!(state.place().cloned(), place);
    }

    #[test]
    fn a_started_container_is_shown_selected_and_its_engine_reloaded() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let effects = effects(on_run_outcome(
            &mut state,
            RunOutcome::Started {
                engine: docker().address().clone(),
                id: String::from("4f1c2b9a8e7d"),
            },
        ));

        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Containers
            ))
        );
        assert_eq!(selected_container(&state), Some("4f1c2b9a8e7d"));
        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
    }

    #[test]
    fn a_volume_being_created_disables_prune_on_volumes() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Volumes);
        let is_prune_enabled = |state: &ContainersState| {
            PruneViewModel::from_state(state.actions(), &state.actions_ctx())
                .map(|vm| vm.is_enabled)
        };
        let before = is_prune_enabled(&state);

        let _ = on_create_outcome(
            &mut state,
            CreateOutcome::ActionRequested(ActionRequest::new(
                docker(),
                EngineAction::CreateVolume {
                    name: String::from("otty-fx-vol"),
                    driver: String::from("local"),
                },
                String::from("otty-fx-vol"),
            )),
        );

        assert_eq!(before, Some(true));
        assert_eq!(is_prune_enabled(&state), Some(false));
    }

    #[test]
    fn a_done_action_reloads_its_engine_only_while_containers_mode_is_active() {
        let mut state = loaded_state();
        let done = || ActionsOutcome::Done {
            engine: docker().address().clone(),
        };

        let active = effects(on_actions_outcome(&mut state, done()));
        let _ = reduce(&mut state, ContainersIntent::Leave);
        let inactive = effects(on_actions_outcome(&mut state, done()));

        assert_eq!(active, vec![ContainersEffect::LoadLists(docker())]);
        assert!(inactive.is_empty());
    }

    #[test]
    fn a_failed_action_tells_the_user() {
        let mut state = loaded_state();

        let effects = effects(on_actions_outcome(
            &mut state,
            ActionsOutcome::Failed(String::from(
                "Could not stop otty-fx-running",
            )),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::Failed(String::from(
                "Could not stop otty-fx-running"
            ))]
        );
    }

    /// A failure is told also after the user left Containers mode, unlike a
    /// reload: the toast shows over any mode.
    #[test]
    fn a_failed_action_tells_the_user_after_leaving_containers_mode() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let effects = effects(on_actions_outcome(
            &mut state,
            ActionsOutcome::Failed(String::from("Could not stop web-1")),
        ));

        assert_eq!(
            effects,
            vec![ContainersEffect::Failed(String::from(
                "Could not stop web-1"
            ))]
        );
    }

    #[test]
    fn a_run_request_opens_the_run_form_on_that_image() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let effects = effects(on_actions_outcome(
            &mut state,
            ActionsOutcome::RunRequested {
                engine: docker(),
                image: String::from("busybox:1.37"),
            },
        ));

        assert_eq!(
            state.run().form().map(|form| form.image()),
            Some("busybox:1.37")
        );
        assert_eq!(
            effects,
            vec![ContainersEffect::Run(RunEffect::LoadImageDefaults {
                engine: docker(),
                reference: String::from("busybox:1.37"),
            })]
        );
    }

    #[test]
    fn a_pull_request_opens_the_pull_dialog_filled_in() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let effects = effects(on_actions_outcome(
            &mut state,
            ActionsOutcome::PullRequested {
                engine: docker(),
                image: String::from("busybox"),
                tag: String::from("1.37"),
            },
        ));

        let dialog = FormDialogViewModel::from_state(state.create());
        assert!(effects.is_empty());
        assert_eq!(
            dialog.as_ref().map(|vm| vm.title),
            Some(i18n::t(Key::ContainersPullImage))
        );
        assert_eq!(
            dialog.map(|vm| {
                vm.fields
                    .iter()
                    .map(|field| field.value.to_string())
                    .collect::<Vec<_>>()
            }),
            Some(vec![String::from("busybox"), String::from("1.37")])
        );
    }
}
