use std::fmt;

use otty_containers::{
    Container, EngineAddress, EngineError, EngineInfo, Image, Network, Volume,
};
use otty_ui_kit::components::tag_search::TagSearch;
use thiserror::Error;

use super::sort::{
    TableSort, sort_containers, sort_images, sort_networks, sort_volumes,
};
use super::table_search::{
    container_row, image_row, keys, network_row, volume_row,
};
use crate::i18n::{self, Key};

/// How `docker images` names an image without a tag.
const DANGLING_IMAGE_NAME: &str = "<none>:<none>";
/// How `docker images` shows a repository or a tag an image does not have.
const NO_IMAGE_PART: &str = "<none>";
/// The tag a pull uses unless another one is typed.
pub(crate) const DEFAULT_TAG: &str = "latest";
/// The inspector tabs of a container.
const CONTAINER_TABS: [InspectorTab; 3] = [
    InspectorTab::Overview,
    InspectorTab::Logs,
    InspectorTab::Inspect,
];
/// The inspector tabs of an image, a volume or a network: they have no logs.
const OTHER_TABS: [InspectorTab; 2] =
    [InspectorTab::Overview, InspectorTab::Inspect];

/// What the main area of Containers mode lists (spec Decision 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ResourceKind {
    Containers,
    Images,
    Volumes,
    Networks,
}

impl ResourceKind {
    /// Every resource, in the order of the engine tree.
    pub(crate) const ALL: [Self; 4] = [
        Self::Containers,
        Self::Images,
        Self::Volumes,
        Self::Networks,
    ];

    /// Translation key of the tree row and the main area title.
    pub(crate) fn title_key(self) -> Key {
        match self {
            Self::Containers => Key::ContainersContainers,
            Self::Images => Key::ContainersImages,
            Self::Volumes => Key::ContainersVolumes,
            Self::Networks => Key::ContainersNetworks,
        }
    }

    /// Translation key of the create button above the table.
    pub(crate) fn create_key(self) -> Key {
        match self {
            Self::Containers => Key::ContainersRunContainer,
            Self::Images => Key::ContainersPullImage,
            Self::Volumes => Key::ContainersCreateVolume,
            Self::Networks => Key::ContainersCreateNetwork,
        }
    }

    /// Translation key of the table message when the list is empty.
    pub(crate) fn empty_key(self) -> Key {
        match self {
            Self::Containers => Key::ContainersNoContainers,
            Self::Images => Key::ContainersNoImages,
            Self::Volumes => Key::ContainersNoVolumes,
            Self::Networks => Key::ContainersNoNetworks,
        }
    }
}

/// A page of Containers mode: one list of one engine, such as the containers of
/// Docker or the images of Podman. Each page has its own search and sort
/// (spec Decision 21).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Place {
    engine: EngineAddress,
    resource: ResourceKind,
}

impl Place {
    /// Where the engine listens; it tells the engines apart.
    pub(crate) fn engine(&self) -> &EngineAddress {
        &self.engine
    }

    /// The list the main area shows.
    pub(crate) fn resource(&self) -> ResourceKind {
        self.resource
    }

    /// The `resource` list of the engine at `engine`.
    pub(crate) fn new(engine: EngineAddress, resource: ResourceKind) -> Self {
        Self { engine, resource }
    }
}

/// One tab of the inspector. The open tab belongs to the widget, not to the
/// inspector: opening another list goes back to Overview, and a list the view
/// moved to by itself keeps the tab.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    /// Key facts of the selected row.
    #[default]
    Overview,
    /// The followed logs of the selected container.
    Logs,
    /// The engine's inspect answer of the selected row, as a tree or JSON.
    Inspect,
}

impl InspectorTab {
    /// The tabs of `resource`, in order: only containers have Logs.
    pub(crate) fn of(resource: ResourceKind) -> &'static [Self] {
        match resource {
            ResourceKind::Containers => &CONTAINER_TABS,
            _ => &OTHER_TABS,
        }
    }

    /// Translation key of the tab label.
    fn label_key(self) -> Key {
        match self {
            Self::Overview => Key::ContainersTabOverview,
            Self::Logs => Key::ContainersTabLogs,
            Self::Inspect => Key::ContainersTabInspect,
        }
    }
}

impl fmt::Display for InspectorTab {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(i18n::t(self.label_key()))
    }
}

/// Why something the Containers widget asked for failed. It holds the text to
/// show, because the intents that carry it must stay `Clone + PartialEq`,
/// which the engine's own error is not.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub(crate) struct ContainersError(String);

#[cfg(test)]
impl ContainersError {
    /// An error showing `text`.
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_string())
    }
}

impl From<EngineError> for ContainersError {
    /// The engine's own words when it refused a request, otherwise the whole
    /// error text.
    fn from(err: EngineError) -> Self {
        Self(err.message())
    }
}

/// The four lists of one engine as one load returned them. Each list
/// succeeds or fails on its own; the error is the engine's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedLists {
    pub(crate) containers: Result<Vec<Container>, ContainersError>,
    pub(crate) images: Result<Vec<Image>, ContainersError>,
    pub(crate) volumes: Result<Vec<Volume>, ContainersError>,
    pub(crate) networks: Result<Vec<Network>, ContainersError>,
}

impl LoadedLists {
    /// Every list failed with the same `error`, for example when the
    /// connection could not be made.
    pub(crate) fn failed(error: &ContainersError) -> Self {
        Self {
            containers: Err(error.clone()),
            images: Err(error.clone()),
            volumes: Err(error.clone()),
            networks: Err(error.clone()),
        }
    }
}

/// One list of an engine as a reload returned it, or the engine's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoadedList {
    Containers(Result<Vec<Container>, ContainersError>),
    Images(Result<Vec<Image>, ContainersError>),
    Volumes(Result<Vec<Volume>, ContainersError>),
    Networks(Result<Vec<Network>, ContainersError>),
}

/// One engine call the app makes for an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EngineAction {
    /// Start the container with this id.
    StartContainer(String),
    StopContainer(String),
    RestartContainer(String),
    PauseContainer(String),
    UnpauseContainer(String),
    /// Remove the container; `force` kills a running one first.
    RemoveContainer {
        id: String,
        force: bool,
    },
    /// Remove every stopped container.
    PruneContainers,
    /// Remove this image name or id; `force` goes past a container that uses
    /// the image, see `ImageRemoval`.
    RemoveImage {
        reference: String,
        force: bool,
    },
    /// Remove every dangling image no container uses.
    PruneImages,
    /// Create a volume; a blank name lets the engine pick one.
    CreateVolume {
        name: String,
        driver: String,
    },
    /// Remove the volume with this name.
    RemoveVolume(String),
    /// Remove the anonymous volumes no container uses; `include_named` adds
    /// the named ones.
    PruneVolumes {
        include_named: bool,
    },
    CreateNetwork {
        name: String,
        driver: String,
    },
    /// Remove the network with this id.
    RemoveNetwork(String),
    /// Remove every user network no container uses.
    PruneNetworks,
}

impl EngineAction {
    /// The list the action changes.
    pub(crate) fn resource(&self) -> ResourceKind {
        match self {
            Self::RemoveImage { .. } | Self::PruneImages => {
                ResourceKind::Images
            },
            Self::CreateVolume { .. }
            | Self::RemoveVolume(_)
            | Self::PruneVolumes { .. } => ResourceKind::Volumes,
            Self::CreateNetwork { .. }
            | Self::RemoveNetwork(_)
            | Self::PruneNetworks => ResourceKind::Networks,
            _ => ResourceKind::Containers,
        }
    }

    /// The id of the row the action works on; `None` for a prune, which
    /// works on the whole list, and for a create.
    pub(crate) fn target(&self) -> Option<&str> {
        match self {
            Self::StartContainer(id)
            | Self::StopContainer(id)
            | Self::RestartContainer(id)
            | Self::PauseContainer(id)
            | Self::UnpauseContainer(id)
            | Self::RemoveContainer { id, .. }
            | Self::RemoveImage { reference: id, .. }
            | Self::RemoveVolume(id)
            | Self::RemoveNetwork(id) => Some(id),
            _ => None,
        }
    }

    /// Translation key of the notice shown when the engine refuses it.
    pub(crate) fn failure_key(&self) -> Key {
        match self {
            Self::StartContainer(_) => Key::TplContainersStartFailed,
            Self::StopContainer(_) => Key::TplContainersStopFailed,
            Self::RestartContainer(_) => Key::TplContainersRestartFailed,
            Self::PauseContainer(_) => Key::TplContainersPauseFailed,
            Self::UnpauseContainer(_) => Key::TplContainersUnpauseFailed,
            Self::RemoveContainer { .. } => {
                Key::TplContainersRemoveContainerFailed
            },
            Self::PruneContainers => Key::TplContainersPruneContainersFailed,
            Self::RemoveImage { .. } => Key::TplContainersRemoveImageFailed,
            Self::PruneImages => Key::TplContainersPruneImagesFailed,
            Self::CreateVolume { .. } => Key::TplContainersCreateVolumeFailed,
            Self::RemoveVolume(_) => Key::TplContainersRemoveVolumeFailed,
            Self::PruneVolumes { .. } => Key::TplContainersPruneVolumesFailed,
            Self::CreateNetwork { .. } => Key::TplContainersCreateNetworkFailed,
            Self::RemoveNetwork(_) => Key::TplContainersRemoveNetworkFailed,
            Self::PruneNetworks => Key::TplContainersPruneNetworksFailed,
        }
    }

    /// The same action with the confirmation switch applied: it forces a
    /// container or image removal and adds named volumes to a volume prune.
    /// Other actions have no switch.
    fn with_switch(self, is_on: bool) -> Self {
        match self {
            Self::RemoveContainer { id, .. } => {
                Self::RemoveContainer { id, force: is_on }
            },
            Self::RemoveImage { reference, .. } => Self::RemoveImage {
                reference,
                force: is_on,
            },
            Self::PruneVolumes { .. } => Self::PruneVolumes {
                include_named: is_on,
            },
            other => other,
        }
    }
}

/// An action on one engine, with the name of its row for the failure
/// notice (empty for a prune).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionRequest {
    engine: EngineInfo,
    action: EngineAction,
    name: String,
}

impl ActionRequest {
    /// The engine the action runs on.
    pub(crate) fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The engine call.
    pub(crate) fn action(&self) -> &EngineAction {
        &self.action
    }

    /// The name of the row, as the table shows it.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// An `action` on `engine` for the row called `name`.
    pub(crate) fn new(
        engine: EngineInfo,
        action: EngineAction,
        name: String,
    ) -> Self {
        Self {
            engine,
            action,
            name,
        }
    }

    /// Whether the request works on the row `target` (or, with `None`, on
    /// the whole list) of the `resource` list of the engine at `address`.
    pub(crate) fn acts_on(
        &self,
        address: &EngineAddress,
        resource: ResourceKind,
        target: Option<&str>,
    ) -> bool {
        self.engine.address() == address
            && self.action.resource() == resource
            && self.action.target() == target
    }

    /// The same request with the confirmation switch applied to its action;
    /// see `EngineAction::with_switch`.
    pub(crate) fn with_switch(self, is_on: bool) -> Self {
        Self {
            action: self.action.with_switch(is_on),
            ..self
        }
    }
}

/// The list the main area shows, lent by the state to a part: its engine,
/// which list it is, the four loaded lists of that engine and the id of the
/// selected row.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OpenList<'a> {
    /// The engine of the open list.
    pub(crate) engine: &'a EngineInfo,
    /// Which list of the engine is open.
    pub(crate) resource: ResourceKind,
    /// The engine's loaded containers; empty while they load or failed.
    pub(crate) containers: &'a [Container],
    /// The engine's loaded images.
    pub(crate) images: &'a [Image],
    /// The engine's loaded volumes.
    pub(crate) volumes: &'a [Volume],
    /// The engine's loaded networks.
    pub(crate) networks: &'a [Network],
    /// The id of the selected row of the open list; an image row goes by its
    /// row key.
    pub(crate) selected_id: Option<&'a str>,
}

impl<'a> OpenList<'a> {
    /// Return the selected row; `None` when nothing is selected or the row
    /// is not in the loaded list.
    pub(crate) fn selected_row(&self) -> Option<ListRow<'a>> {
        self.find(self.selected_id?)
    }

    /// Return the row with `id` in the open list: a container or network id,
    /// an image row key or a volume name.
    pub(crate) fn find(&self, id: &str) -> Option<ListRow<'a>> {
        let row = match self.resource {
            ResourceKind::Containers => ListRow::Container(
                self.containers.iter().find(|row| row.id() == id)?,
            ),
            ResourceKind::Images => ListRow::Image(
                self.images.iter().find(|row| row.row_key() == id)?,
            ),
            ResourceKind::Volumes => ListRow::Volume(
                self.volumes.iter().find(|row| row.name() == id)?,
            ),
            ResourceKind::Networks => ListRow::Network(
                self.networks.iter().find(|row| row.id() == id)?,
            ),
        };

        Some(row)
    }
}

/// A row of the open list, most often the selected one.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ListRow<'a> {
    Container(&'a Container),
    Image(&'a Image),
    Volume(&'a Volume),
    Network(&'a Network),
}

impl ListRow<'_> {
    /// Return the list the row is in.
    pub(crate) fn resource(&self) -> ResourceKind {
        match self {
            Self::Container(_) => ResourceKind::Containers,
            Self::Image(_) => ResourceKind::Images,
            Self::Volume(_) => ResourceKind::Volumes,
            Self::Network(_) => ResourceKind::Networks,
        }
    }

    /// Return what inspects the row: container, image or network id, or volume
    /// name. The rows of one image share their inspect answer.
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Container(container) => container.id(),
            Self::Image(image) => image.id(),
            Self::Volume(volume) => volume.name(),
            Self::Network(network) => network.id(),
        }
    }
}

/// Return the containers that pass `search`, ordered by `sort`. Ages are
/// counted up to `now` (Unix seconds).
pub(crate) fn shown_containers<'a>(
    containers: &'a [Container],
    search: &TagSearch,
    now: i64,
    sort: TableSort,
) -> Vec<&'a Container> {
    let keys = keys(ResourceKind::Containers);
    let filters_rows = search.filters_rows(&keys);

    let mut shown: Vec<&Container> = containers
        .iter()
        .filter(|item| {
            !filters_rows || search.passes(&keys, &container_row(item, now))
        })
        .collect();
    sort_containers(&mut shown, sort);

    shown
}

/// Return the images that pass `search`, ordered by `sort`.
pub(crate) fn shown_images<'a>(
    images: &'a [Image],
    search: &TagSearch,
    now: i64,
    sort: TableSort,
) -> Vec<&'a Image> {
    let keys = keys(ResourceKind::Images);
    let filters_rows = search.filters_rows(&keys);

    let mut shown: Vec<&Image> = images
        .iter()
        .filter(|item| {
            !filters_rows || search.passes(&keys, &image_row(item, now))
        })
        .collect();
    sort_images(&mut shown, sort);

    shown
}

/// Return the volumes that pass `search`, ordered by `sort`.
pub(crate) fn shown_volumes<'a>(
    volumes: &'a [Volume],
    search: &TagSearch,
    now: i64,
    sort: TableSort,
) -> Vec<&'a Volume> {
    let keys = keys(ResourceKind::Volumes);
    let filters_rows = search.filters_rows(&keys);

    let mut shown: Vec<&Volume> = volumes
        .iter()
        .filter(|item| {
            !filters_rows || search.passes(&keys, &volume_row(item, now))
        })
        .collect();
    sort_volumes(&mut shown, sort);

    shown
}

/// Return the networks that pass `search`, ordered by `sort`.
pub(crate) fn shown_networks<'a>(
    networks: &'a [Network],
    search: &TagSearch,
    now: i64,
    sort: TableSort,
) -> Vec<&'a Network> {
    let keys = keys(ResourceKind::Networks);
    let filters_rows = search.filters_rows(&keys);

    let mut shown: Vec<&Network> = networks
        .iter()
        .filter(|item| {
            !filters_rows || search.passes(&keys, &network_row(item, now))
        })
        .collect();
    sort_networks(&mut shown, sort);

    shown
}

/// The name of an image row: its reference, or `<none>:<none>` for a dangling
/// image, as `docker images` shows it.
pub(crate) fn image_name(image: &Image) -> &str {
    image.reference().unwrap_or(DANGLING_IMAGE_NAME)
}

/// The repository of an image row as `docker images` shows it.
pub(crate) fn image_repository(image: &Image) -> &str {
    image.repository().unwrap_or(NO_IMAGE_PART)
}

/// The tag of an image row as `docker images` shows it.
pub(crate) fn image_tag(image: &Image) -> &str {
    image.tag().unwrap_or(NO_IMAGE_PART)
}

/// Split an image reference into its name and tag:
/// `docker.io/library/busybox:1.37` gives `docker.io/library/busybox` and
/// `1.37`. A colon before the last `/` belongs to a registry port, so
/// `localhost:5000/app` has the default tag.
pub(crate) fn split_reference(reference: &str) -> (&str, &str) {
    let name_start = reference.rfind('/').map_or(0, |slash| slash + 1);

    match reference[name_start..].rfind(':') {
        Some(colon) => {
            let at = name_start + colon;
            (&reference[..at], &reference[at + 1..])
        },
        None => (reference, DEFAULT_TAG),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_containers::{EngineKind, fixtures};
    use otty_ui_kit::components::tag_search::{
        TagKey, TagSchema, TagSearchAction,
    };

    use super::*;

    const NOW: i64 = 1_793_557_960;

    /// A search with `tags` and `text` typed in the field.
    fn search(tags: &[(&'static str, &str)], text: &str) -> TagSearch {
        let schema =
            TagSchema::new(vec![TagKey::new("name", "name")], Vec::new());
        let mut search = TagSearch::default();
        for (key, value) in tags {
            add_tag(&mut search, key, value);
        }
        let _ = search
            .perform(TagSearchAction::InputChanged(text.to_string()), &schema);

        search
    }

    /// Adds `key:value` as Enter does: several tags of one key stay.
    fn add_tag(search: &mut TagSearch, key: &'static str, value: &str) {
        let schema = TagSchema::new(vec![TagKey::new(key, key)], Vec::new());

        for action in [
            TagSearchAction::InputChanged(format!("{key}:{value}")),
            TagSearchAction::Apply,
        ] {
            let _ = search.perform(action, &schema);
        }
    }

    fn request(action: EngineAction) -> ActionRequest {
        ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            action,
            String::from("otty-fx-running"),
        )
    }

    fn container_names(search: &TagSearch) -> Vec<String> {
        let resources = fixtures::resources(EngineKind::Docker);
        let sort = TableSort::default_for(ResourceKind::Containers);

        shown_containers(resources.containers(), search, NOW, sort)
            .iter()
            .map(|container| container.name().to_string())
            .collect()
    }

    #[test]
    fn plain_text_matches_part_of_the_name_ignoring_case() {
        assert_eq!(
            container_names(&search(&[], " EXIT ")),
            vec!["otty-fx-exit0", "otty-fx-exit3"]
        );
    }

    #[test]
    fn plain_text_that_looks_like_a_tag_is_still_a_name_filter() {
        assert!(container_names(&search(&[], "redis:7")).is_empty());
    }

    #[test]
    fn an_empty_search_keeps_every_row() {
        let resources = fixtures::resources(EngineKind::Docker);

        assert_eq!(
            container_names(&search(&[], "")).len(),
            resources.containers().len()
        );
    }

    #[test]
    fn a_compose_tag_limits_the_containers_to_the_project() {
        assert_eq!(
            container_names(&search(&[("compose", "otty-fx")], "")),
            vec!["otty-fx-web-1"]
        );
        assert!(
            container_names(&search(&[("compose", "nope")], "")).is_empty()
        );
    }

    #[test]
    fn tags_of_different_keys_must_all_match() {
        let found = container_names(&search(
            &[("status", "running"), ("ports", "18080")],
            "",
        ));

        assert_eq!(found, vec!["otty-fx-running"]);
    }

    #[test]
    fn tags_of_one_key_match_if_any_does() {
        let found = container_names(&search(
            &[("name", "exit3"), ("name", "paused")],
            "",
        ));

        assert_eq!(found, vec!["otty-fx-paused", "otty-fx-exit3"]);
    }

    #[test]
    fn tags_and_the_name_text_must_both_match() {
        assert!(
            container_names(&search(&[("compose", "otty-fx")], "exit"))
                .is_empty()
        );
    }

    #[test]
    fn a_tag_on_ports_matches_any_port_of_the_container() {
        assert_eq!(
            container_names(&search(&[("ports", "18080:80")], "")),
            vec!["otty-fx-running"]
        );
    }

    #[test]
    fn each_list_is_filtered_by_its_own_keys_and_names() {
        let resources = fixtures::resources(EngineKind::Docker);

        let images = shown_images(
            resources.images(),
            &search(&[("repository", "busybox")], ""),
            NOW,
            TableSort::default_for(ResourceKind::Images),
        );
        let volumes = shown_volumes(
            resources.volumes(),
            &search(&[], "fx-data"),
            NOW,
            TableSort::default_for(ResourceKind::Volumes),
        );
        let networks = shown_networks(
            resources.networks(),
            &search(&[("driver", "null")], ""),
            NOW,
            TableSort::default_for(ResourceKind::Networks),
        );

        assert_eq!(images.len(), 1);
        assert_eq!(volumes[0].name(), "otty-fx-data");
        assert_eq!(networks.len(), 1);
    }

    #[test]
    fn an_image_is_named_by_its_reference_or_none() {
        let resources = fixtures::resources(EngineKind::Docker);

        let names: Vec<&str> =
            resources.images().iter().map(image_name).collect();

        assert_eq!(
            names,
            vec!["<none>:<none>", "otty-fx-img:1", "busybox:1.37"]
        );
    }

    #[test]
    fn a_tag_on_the_image_tag_key_keeps_the_rows_with_that_tag() {
        let images = fixtures::image_rows(
            "sha256:app",
            &["a:1", "a:2", "b:1"],
            &[],
            &[],
        );
        let sort = TableSort::default_for(ResourceKind::Images);

        let shown = shown_images(
            &images,
            &search(&[("image-tag", "1")], ""),
            NOW,
            sort,
        );

        let names: Vec<&str> =
            shown.iter().map(|row| image_name(row)).collect();
        assert_eq!(names, vec!["a:1", "b:1"]);
    }

    #[test]
    fn plain_text_finds_an_image_by_repository_and_tag_together() {
        let images =
            fixtures::image_rows("sha256:app", &["a:1", "a:2"], &[], &[]);
        let sort = TableSort::default_for(ResourceKind::Images);

        let shown = shown_images(&images, &search(&[], "a:2"), NOW, sort);

        let names: Vec<&str> =
            shown.iter().map(|row| image_name(row)).collect();
        assert_eq!(names, vec!["a:2"]);
    }

    #[test]
    fn a_failed_load_fails_every_list_with_the_message() {
        let lists = LoadedLists::failed(&ContainersError::new("refused"));

        assert_eq!(lists.containers, Err(ContainersError::new("refused")));
        assert_eq!(lists.images, Err(ContainersError::new("refused")));
        assert_eq!(lists.volumes, Err(ContainersError::new("refused")));
        assert_eq!(lists.networks, Err(ContainersError::new("refused")));
    }

    #[test]
    fn it_shows_its_text_as_it_is() {
        let error = ContainersError::new("port is already allocated");

        assert_eq!(error.to_string(), "port is already allocated");
    }

    #[test]
    fn an_engine_error_becomes_the_text_of_its_message() {
        let engine_error = EngineError::PathNotUtf8(PathBuf::from("/run/x"));
        let message = engine_error.message();

        let error = ContainersError::from(engine_error);

        assert_eq!(error.to_string(), message);
    }

    #[test]
    fn a_reference_splits_into_name_and_tag() {
        assert_eq!(
            split_reference("docker.io/library/busybox:1.37"),
            ("docker.io/library/busybox", "1.37")
        );
        assert_eq!(split_reference("nginx"), ("nginx", DEFAULT_TAG));
        assert_eq!(
            split_reference("localhost:5000/app"),
            ("localhost:5000/app", DEFAULT_TAG)
        );
        assert_eq!(
            split_reference("localhost:5000/app:v2"),
            ("localhost:5000/app", "v2")
        );
    }

    #[test]
    fn an_action_without_a_switch_ignores_the_switch() {
        let action = EngineAction::StopContainer(String::from("web"));

        assert_eq!(action.clone().with_switch(true), action);
    }

    #[test]
    fn each_action_names_its_list_and_its_row() {
        let id = String::from("c1");
        let actions = [
            EngineAction::StartContainer(id.clone()),
            EngineAction::StopContainer(id.clone()),
            EngineAction::RestartContainer(id.clone()),
            EngineAction::PauseContainer(id.clone()),
            EngineAction::UnpauseContainer(id.clone()),
            EngineAction::RemoveContainer {
                id: id.clone(),
                force: true,
            },
        ];

        for action in actions {
            assert_eq!(action.resource(), ResourceKind::Containers);
            assert_eq!(action.target(), Some("c1"), "{action:?}");
        }
        assert_eq!(
            EngineAction::PruneContainers.resource(),
            ResourceKind::Containers
        );
        assert_eq!(EngineAction::PruneContainers.target(), None);
    }

    #[test]
    fn a_request_runs_on_its_engine_list_and_row() {
        let stop = request(EngineAction::StopContainer(String::from("c1")));
        let docker = fixtures::engine(EngineKind::Docker);
        let podman = fixtures::engine(EngineKind::Podman);

        assert!(stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            Some("c1")
        ));
        assert!(!stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            Some("c2")
        ));
        assert!(!stop.acts_on(
            podman.address(),
            ResourceKind::Containers,
            Some("c1")
        ));
        assert!(!stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            None
        ));
    }

    #[test]
    fn every_failure_has_its_own_text() {
        let id = || String::from("c1");
        let keys: Vec<Key> = [
            EngineAction::StartContainer(id()),
            EngineAction::StopContainer(id()),
            EngineAction::RestartContainer(id()),
            EngineAction::PauseContainer(id()),
            EngineAction::UnpauseContainer(id()),
            EngineAction::RemoveContainer {
                id: id(),
                force: false,
            },
            EngineAction::PruneContainers,
            EngineAction::RemoveImage {
                reference: id(),
                force: false,
            },
            EngineAction::PruneImages,
            EngineAction::CreateVolume {
                name: id(),
                driver: id(),
            },
            EngineAction::RemoveVolume(id()),
            EngineAction::PruneVolumes {
                include_named: false,
            },
            EngineAction::CreateNetwork {
                name: id(),
                driver: id(),
            },
            EngineAction::RemoveNetwork(id()),
            EngineAction::PruneNetworks,
        ]
        .iter()
        .map(EngineAction::failure_key)
        .collect();

        for (position, key) in keys.iter().enumerate() {
            assert!(!keys[position + 1..].contains(key), "{key:?} repeats");
        }
    }

    #[test]
    fn image_volume_and_network_actions_name_their_list_and_row() {
        let id = || String::from("x1");
        let cases = [
            (
                EngineAction::RemoveImage {
                    reference: id(),
                    force: true,
                },
                ResourceKind::Images,
                Some("x1"),
            ),
            (EngineAction::PruneImages, ResourceKind::Images, None),
            (
                EngineAction::CreateVolume {
                    name: id(),
                    driver: String::from("local"),
                },
                ResourceKind::Volumes,
                None,
            ),
            (
                EngineAction::RemoveVolume(id()),
                ResourceKind::Volumes,
                Some("x1"),
            ),
            (
                EngineAction::PruneVolumes {
                    include_named: false,
                },
                ResourceKind::Volumes,
                None,
            ),
            (
                EngineAction::CreateNetwork {
                    name: id(),
                    driver: String::from("bridge"),
                },
                ResourceKind::Networks,
                None,
            ),
            (
                EngineAction::RemoveNetwork(id()),
                ResourceKind::Networks,
                Some("x1"),
            ),
            (EngineAction::PruneNetworks, ResourceKind::Networks, None),
        ];

        for (action, resource, target) in cases {
            assert_eq!(action.resource(), resource, "{action:?}");
            assert_eq!(action.target(), target, "{action:?}");
        }
    }

    /// The open `resource` list of the Docker fixture, with `selected_id`.
    fn open_list<'a>(
        resources: &'a otty_containers::Resources,
        engine: &'a EngineInfo,
        resource: ResourceKind,
        selected_id: Option<&'a str>,
    ) -> OpenList<'a> {
        OpenList {
            engine,
            resource,
            containers: resources.containers(),
            images: resources.images(),
            volumes: resources.volumes(),
            networks: resources.networks(),
            selected_id,
        }
    }

    #[test]
    fn the_open_list_finds_a_row_by_the_id_of_its_kind() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = fixtures::engine(EngineKind::Docker);
        let container = &resources.containers()[1];
        let image = &resources.images()[1];
        let volume = &resources.volumes()[0];
        let network = &resources.networks()[0];
        let find = |resource, id| {
            open_list(&resources, &engine, resource, None)
                .find(id)
                .map(|row| (row.resource(), row.id().to_string()))
        };

        let found = [
            find(ResourceKind::Containers, container.id()),
            find(ResourceKind::Images, image.row_key()),
            find(ResourceKind::Volumes, volume.name()),
            find(ResourceKind::Networks, network.id()),
        ];

        assert_eq!(
            found,
            [
                Some((ResourceKind::Containers, container.id().to_string())),
                Some((ResourceKind::Images, image.id().to_string())),
                Some((ResourceKind::Volumes, volume.name().to_string())),
                Some((ResourceKind::Networks, network.id().to_string())),
            ]
        );
        assert!(find(ResourceKind::Images, container.id()).is_none());
    }

    #[test]
    fn the_selected_row_of_the_open_list_must_be_in_its_list() {
        let resources = fixtures::resources(EngineKind::Docker);
        let engine = fixtures::engine(EngineKind::Docker);
        let container = &resources.containers()[2];

        let selected = open_list(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some(container.id()),
        )
        .selected_row()
        .map(|row| row.id().to_string());
        let gone = open_list(
            &resources,
            &engine,
            ResourceKind::Containers,
            Some("gone"),
        )
        .selected_row();
        let none =
            open_list(&resources, &engine, ResourceKind::Containers, None)
                .selected_row();

        assert_eq!(selected, Some(container.id().to_string()));
        assert!(gone.is_none());
        assert!(none.is_none());
    }
}
