use otty_docker::{Container, EngineAddress, Image, Network, Volume};
use otty_ui_kit::components::tag_search::TagSearch;

use super::errors::DockerError;
use super::sort::{
    TableSort, sort_containers, sort_images, sort_networks, sort_volumes,
};
use super::table_search::{
    container_row, image_row, keys, network_row, volume_row,
};
use crate::i18n::Key;

/// How `docker images` names an image without a tag.
const DANGLING_IMAGE_NAME: &str = "<none>:<none>";

/// What the main area of Docker mode lists (spec Decision 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            Self::Containers => Key::DockerContainers,
            Self::Images => Key::DockerImages,
            Self::Volumes => Key::DockerVolumes,
            Self::Networks => Key::DockerNetworks,
        }
    }

    /// Translation key of the create button above the table.
    pub(crate) fn create_key(self) -> Key {
        match self {
            Self::Containers => Key::DockerRunContainer,
            Self::Images => Key::DockerPullImage,
            Self::Volumes => Key::DockerCreateVolume,
            Self::Networks => Key::DockerCreateNetwork,
        }
    }

    /// Translation key of the table message when the list is empty.
    pub(crate) fn empty_key(self) -> Key {
        match self {
            Self::Containers => Key::DockerNoContainers,
            Self::Images => Key::DockerNoImages,
            Self::Volumes => Key::DockerNoVolumes,
            Self::Networks => Key::DockerNoNetworks,
        }
    }
}

/// The open place of Docker mode: which engine and which of its lists the
/// main area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// The four lists of one engine as one load returned them. Each list
/// succeeds or fails on its own; the error is the engine's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedLists {
    pub(crate) containers: Result<Vec<Container>, DockerError>,
    pub(crate) images: Result<Vec<Image>, DockerError>,
    pub(crate) volumes: Result<Vec<Volume>, DockerError>,
    pub(crate) networks: Result<Vec<Network>, DockerError>,
}

impl LoadedLists {
    /// Every list failed with the same `error`, for example when the
    /// connection could not be made.
    pub(crate) fn failed(error: &DockerError) -> Self {
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
    Containers(Result<Vec<Container>, DockerError>),
    Images(Result<Vec<Image>, DockerError>),
    Volumes(Result<Vec<Volume>, DockerError>),
    Networks(Result<Vec<Network>, DockerError>),
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

    let mut shown: Vec<&Container> = containers
        .iter()
        .filter(|item| search.passes(&keys, &container_row(item, now)))
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

    let mut shown: Vec<&Image> = images
        .iter()
        .filter(|item| search.passes(&keys, &image_row(item, now)))
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

    let mut shown: Vec<&Volume> = volumes
        .iter()
        .filter(|item| search.passes(&keys, &volume_row(item, now)))
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

    let mut shown: Vec<&Network> = networks
        .iter()
        .filter(|item| search.passes(&keys, &network_row(item, now)))
        .collect();
    sort_networks(&mut shown, sort);

    shown
}

/// The first tag of an image, or `<none>:<none>` for a dangling one, as
/// `docker images` shows it.
pub(crate) fn image_name(image: &Image) -> &str {
    image
        .tags()
        .first()
        .map_or(DANGLING_IMAGE_NAME, String::as_str)
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};
    use otty_ui_kit::components::tag_search::{
        TagKey, TagSchema, TagSearchAction,
    };

    use super::*;
    use crate::widgets::docker::sort::TableSorts;

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
        let sort = TableSorts::default().get(ResourceKind::Containers);

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
        let sorts = TableSorts::default();

        let images = shown_images(
            resources.images(),
            &search(&[("repository", "busybox")], ""),
            NOW,
            sorts.get(ResourceKind::Images),
        );
        let volumes = shown_volumes(
            resources.volumes(),
            &search(&[], "fx-data"),
            NOW,
            sorts.get(ResourceKind::Volumes),
        );
        let networks = shown_networks(
            resources.networks(),
            &search(&[("driver", "null")], ""),
            NOW,
            sorts.get(ResourceKind::Networks),
        );

        assert_eq!(images.len(), 1);
        assert_eq!(volumes[0].name(), "otty-fx-data");
        assert_eq!(networks.len(), 1);
    }

    #[test]
    fn an_image_is_named_by_its_first_tag_or_none() {
        let resources = fixtures::resources(EngineKind::Docker);

        let names: Vec<&str> =
            resources.images().iter().map(image_name).collect();

        assert_eq!(
            names,
            vec!["<none>:<none>", "otty-fx-img:1", "busybox:1.37"]
        );
    }

    #[test]
    fn a_failed_load_fails_every_list_with_the_message() {
        let lists = LoadedLists::failed(&DockerError::new("refused"));

        assert_eq!(lists.containers, Err(DockerError::new("refused")));
        assert_eq!(lists.images, Err(DockerError::new("refused")));
        assert_eq!(lists.volumes, Err(DockerError::new("refused")));
        assert_eq!(lists.networks, Err(DockerError::new("refused")));
    }
}
