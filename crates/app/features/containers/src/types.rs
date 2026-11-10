use otty_app_i18n::Key;
use otty_containers::{
    Container, EngineAddress, EngineKind, Image, Network, Volume,
};
use otty_ui_kit::components::tag_search::TagSearch;
use thiserror::Error;

use super::sort::{
    TableSort, sort_containers, sort_images, sort_networks, sort_volumes,
};
use super::table_search::{
    container_row, image_row, keys, network_row, volume_row,
};

/// How `docker images` names an image without a tag.
const DANGLING_IMAGE_NAME: &str = "<none>:<none>";
/// How `docker images` shows a repository or a tag an image does not have.
const NO_IMAGE_PART: &str = "<none>";
/// The tag a pull uses unless another one is typed.
pub(crate) const DEFAULT_TAG: &str = "latest";

/// What the main area of Containers mode lists (spec Decision 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// The containers of an engine.
    Containers,
    /// The images of an engine, one row per tag.
    Images,
    /// The volumes of an engine.
    Volumes,
    /// The networks of an engine.
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
    #[expect(dead_code, reason = "issue 14: the create button of each list")]
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
pub struct Place {
    engine: EngineAddress,
    resource: ResourceKind,
}

impl Place {
    /// Where the engine listens; it tells the engines apart.
    pub fn engine(&self) -> &EngineAddress {
        &self.engine
    }

    /// The list the main area shows.
    pub fn resource(&self) -> ResourceKind {
        self.resource
    }

    /// The `resource` list of the engine at `engine`.
    pub fn new(engine: EngineAddress, resource: ResourceKind) -> Self {
        Self { engine, resource }
    }
}

/// Why something the containers feature asked for failed. It holds the text
/// to show, because the intents that carry it must stay `Clone + PartialEq`,
/// which the engine's own error is not.
///
/// The binary builds it from the engine's error: the engine's own words when
/// it refused a request, otherwise the whole error text.
///
/// # Example
///
/// ```
/// use otty_app_containers::ContainersError;
///
/// let error = ContainersError::new("port is already allocated");
///
/// assert_eq!(error.to_string(), "port is already allocated");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct ContainersError(String);

impl ContainersError {
    /// An error showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// The four lists of one engine as one load returned them. Each list
/// succeeds or fails on its own; the error is the engine's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLists {
    /// The containers, or the engine's message.
    pub containers: Result<Vec<Container>, ContainersError>,
    /// The images, or the engine's message.
    pub images: Result<Vec<Image>, ContainersError>,
    /// The volumes, or the engine's message.
    pub volumes: Result<Vec<Volume>, ContainersError>,
    /// The networks, or the engine's message.
    pub networks: Result<Vec<Network>, ContainersError>,
}

impl LoadedLists {
    /// Every list failed with the same `error`, for example when the
    /// connection could not be made.
    pub fn failed(error: &ContainersError) -> Self {
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
pub enum LoadedList {
    /// The containers, or the engine's message.
    Containers(Result<Vec<Container>, ContainersError>),
    /// The images, or the engine's message.
    Images(Result<Vec<Image>, ContainersError>),
    /// The volumes, or the engine's message.
    Volumes(Result<Vec<Volume>, ContainersError>),
    /// The networks, or the engine's message.
    Networks(Result<Vec<Network>, ContainersError>),
}

/// Why no engine can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoEngineCase {
    /// Neither Docker nor Podman is installed.
    NotInstalled,
    /// Docker is installed but no Docker socket answers.
    DockerNotRunning,
    /// Podman is installed but its API service is off.
    PodmanApiOff,
    /// Docker and Podman are both installed and neither answers.
    DockerAndPodmanOff,
    /// A socket exists but this user may not connect to it.
    NoPermission,
    /// A Podman socket exists but this user may not connect to it.
    PodmanNoPermission,
    /// The socket of this engine refused this user, and the other engine is
    /// installed but off.
    NoPermissionAndOtherOff(EngineKind),
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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "issue 14: the pull dialog and the Run form split image references"
    )
)]
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
}
