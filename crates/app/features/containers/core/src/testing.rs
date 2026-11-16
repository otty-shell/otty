//! Helpers that the tests of several view model files share: a state with
//! engines found and their lists loaded, the steps a person takes in the
//! engine tree and the tag search, and the view models built from a state.

use otty_app_i18n::{self as i18n, Key};
use otty_containers::{EngineAddress, EngineInfo, EngineKind, fixtures};
use otty_ui_kit::components::tag_search::TagSearchAction;

use crate::event::ContainersIntent;
use crate::model::{
    EngineTreeViewModel, InspectorViewModel, MainAreaViewModel, TableViewModel,
};
use crate::reducer::reduce;
use crate::state::ContainersState;
use crate::types::{LoadedLists, Place, ResourceKind};

pub(crate) const NOW: i64 = 1_793_557_960;

/// All three parts of Containers mode at [`NOW`].
pub(crate) struct Vm<'a> {
    pub(crate) tree: EngineTreeViewModel,
    pub(crate) main: MainAreaViewModel<'a>,
    pub(crate) inspector: Option<InspectorViewModel>,
}

pub(crate) fn build(state: &ContainersState) -> Vm<'_> {
    Vm {
        tree: EngineTreeViewModel::from_state(state),
        main: MainAreaViewModel::from_state(state, NOW),
        inspector: InspectorViewModel::from_state(state),
    }
}

pub(crate) fn docker() -> EngineInfo {
    fixtures::engine(EngineKind::Docker)
}

pub(crate) fn podman() -> EngineInfo {
    fixtures::engine(EngineKind::Podman)
}

pub(crate) fn lists(kind: EngineKind) -> LoadedLists {
    let resources = fixtures::resources(kind);

    LoadedLists {
        containers: Ok(resources.containers().to_vec()),
        images: Ok(resources.images().to_vec()),
        volumes: Ok(resources.volumes().to_vec()),
        networks: Ok(resources.networks().to_vec()),
    }
}

pub(crate) fn found(kinds: &[EngineKind]) -> ContainersState {
    let mut state = ContainersState::default();
    let _ = reduce(&mut state, ContainersIntent::Enter);
    let _ = reduce(
        &mut state,
        ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
    );
    state
}

pub(crate) fn load(
    state: &mut ContainersState,
    info: &EngineInfo,
    lists: LoadedLists,
) {
    let _ = reduce(
        state,
        ContainersIntent::ListsLoaded {
            engine: info.address().clone(),
            lists,
        },
    );
}

pub(crate) fn open(state: &mut ContainersState, resource: ResourceKind) {
    let place = Place::new(docker().address().clone(), resource);
    let _ = reduce(state, ContainersIntent::Open(place));
}

pub(crate) fn table<'a>(vm: &'a Vm<'_>) -> &'a TableViewModel<'a> {
    match &vm.main {
        MainAreaViewModel::Table(table) => table,
        other => panic!("expected a table, got {other:?}"),
    }
}

/// The labels of the tree rows, top to bottom.
pub(crate) fn labels(state: &ContainersState) -> Vec<String> {
    EngineTreeViewModel::from_state(state)
        .rows
        .into_iter()
        .map(|row| row.label)
        .collect()
}

/// The labels of the highlighted tree rows with the engine of each.
pub(crate) fn highlighted(
    state: &ContainersState,
) -> Vec<(String, EngineAddress)> {
    EngineTreeViewModel::from_state(state)
        .rows
        .into_iter()
        .filter(|row| row.is_selected)
        .map(|row| (row.label, row.place.engine().clone()))
        .collect()
}

pub(crate) fn press_root(state: &mut ContainersState, info: &EngineInfo) {
    let _ = reduce(
        state,
        ContainersIntent::ToggleEngine(info.address().clone()),
    );
}

/// The labels of an open engine: its root and its four lists.
pub(crate) fn open_labels(root: &str) -> Vec<String> {
    [
        root,
        i18n::t(Key::ContainersContainers),
        i18n::t(Key::ContainersImages),
        i18n::t(Key::ContainersVolumes),
        i18n::t(Key::ContainersNetworks),
    ]
    .map(str::to_string)
    .to_vec()
}

pub(crate) fn search_does(
    state: &mut ContainersState,
    action: TagSearchAction,
) {
    let _ = reduce(state, ContainersIntent::TagSearch(action));
}

pub(crate) fn type_text(state: &mut ContainersState, text: &str) {
    search_does(state, TagSearchAction::InputChanged(text.to_string()));
}

/// Adds the typed `key:text` as it is, as Enter does with no suggestion.
pub(crate) fn add_tag(state: &mut ContainersState, text: &str) {
    type_text(state, text);
    search_does(state, TagSearchAction::Close);
    search_does(state, TagSearchAction::Apply);
}

/// Docker with one image of two tags, its images open.
pub(crate) fn two_tags_of_one_image() -> ContainersState {
    let mut state = found(&[EngineKind::Docker]);
    let mut loaded = lists(EngineKind::Docker);
    loaded.images = Ok(fixtures::image_rows(
        "sha256:app",
        &["app:1", "app:2"],
        &[],
        &[],
    ));
    load(&mut state, &docker(), loaded);
    open(&mut state, ResourceKind::Images);
    state
}
