use std::collections::BTreeMap;

use otty_docker::{Container, EngineAddress, Image, Network, Volume};

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
    /// The Compose project the containers are limited to.
    compose_project: Option<String>,
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

    /// The Compose project the containers are limited to; `None` shows all
    /// of them.
    pub(crate) fn compose_project(&self) -> Option<&str> {
        self.compose_project.as_deref()
    }

    /// The `resource` list of the engine at `engine`.
    pub(crate) fn new(engine: EngineAddress, resource: ResourceKind) -> Self {
        Self {
            engine,
            resource,
            compose_project: None,
        }
    }

    /// The containers of the engine at `engine` that belong to the Compose
    /// project `project`.
    pub(crate) fn compose(engine: EngineAddress, project: &str) -> Self {
        Self {
            engine,
            resource: ResourceKind::Containers,
            compose_project: Some(project.to_string()),
        }
    }
}

/// The four lists of one engine as one load returned them. Each list
/// succeeds or fails on its own; the error is the engine's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedLists {
    pub(crate) containers: Result<Vec<Container>, String>,
    pub(crate) images: Result<Vec<Image>, String>,
    pub(crate) volumes: Result<Vec<Volume>, String>,
    pub(crate) networks: Result<Vec<Network>, String>,
}

impl LoadedLists {
    /// Every list failed with the same `message`, for example when the
    /// connection could not be made.
    pub(crate) fn failed(message: &str) -> Self {
        Self {
            containers: Err(message.to_string()),
            images: Err(message.to_string()),
            volumes: Err(message.to_string()),
            networks: Err(message.to_string()),
        }
    }
}

/// Return the containers whose name contains `filter` and, when `project`
/// is set, that belong to that Compose project.
pub(crate) fn shown_containers<'a>(
    containers: &'a [Container],
    filter: &str,
    project: Option<&str>,
) -> Vec<&'a Container> {
    let mut shown = filtered(containers, filter, container_name);
    if let Some(project) = project {
        shown.retain(|container| container.compose_project() == Some(project));
    }

    shown
}

/// The Compose projects of `containers` with their container counts,
/// ordered by project name. Containers outside any project are skipped.
pub(crate) fn compose_projects(containers: &[Container]) -> Vec<(&str, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for project in containers.iter().filter_map(Container::compose_project) {
        *counts.entry(project).or_default() += 1;
    }

    counts.into_iter().collect()
}

/// The name a row is filtered by and shown under: the container, volume or
/// network name, or the first tag of an image.
pub(crate) fn container_name(container: &Container) -> &str {
    container.name()
}

/// The first tag of an image, or `<none>:<none>` for a dangling one, as
/// `docker images` shows it.
pub(crate) fn image_name(image: &Image) -> &str {
    image
        .tags()
        .first()
        .map_or(DANGLING_IMAGE_NAME, String::as_str)
}

/// The name of a volume; volumes have no separate id.
pub(crate) fn volume_name(volume: &Volume) -> &str {
    volume.name()
}

/// The name of a network.
pub(crate) fn network_name(network: &Network) -> &str {
    network.name()
}

/// Return the items whose name contains `filter`, ignoring case, in their
/// order. An empty filter keeps every item.
pub(crate) fn filtered<'a, T>(
    items: &'a [T],
    filter: &str,
    name_of: fn(&T) -> &str,
) -> Vec<&'a T> {
    let needle = filter.trim().to_lowercase();

    items
        .iter()
        .filter(|item| name_of(item).to_lowercase().contains(&needle))
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    #[test]
    fn the_filter_matches_part_of_the_name_ignoring_case() {
        let resources = fixtures::resources(EngineKind::Docker);

        let rows = filtered(resources.containers(), " EXIT ", container_name);

        let names: Vec<&str> = rows.iter().map(|c| c.name()).collect();
        assert_eq!(names, vec!["otty-fx-exit3", "otty-fx-exit0"]);
    }

    #[test]
    fn containers_are_limited_to_a_compose_project_and_the_filter() {
        let resources = fixtures::resources(EngineKind::Docker);
        let containers = resources.containers();

        let all = shown_containers(containers, "", None);
        let project = shown_containers(containers, "", Some("otty-fx"));
        let other = shown_containers(containers, "", Some("nope"));
        let no_match = shown_containers(containers, "exit", Some("otty-fx"));

        assert_eq!(all.len(), containers.len());
        let names: Vec<&str> = project.iter().map(|c| c.name()).collect();
        assert_eq!(names, vec!["otty-fx-web-1"]);
        assert!(other.is_empty());
        assert!(no_match.is_empty());
    }

    #[test]
    fn compose_projects_are_counted_by_name_and_skip_loose_containers() {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut containers = resources.containers().to_vec();
        let web = containers
            .iter()
            .find(|c| c.compose_project().is_some())
            .expect("fixture has a compose container")
            .clone();
        containers.push(web);

        let projects = compose_projects(&containers);

        assert_eq!(projects, vec![("otty-fx", 2)]);
        assert!(compose_projects(&[]).is_empty());
    }

    #[test]
    fn an_empty_filter_keeps_every_row() {
        let resources = fixtures::resources(EngineKind::Docker);

        let rows = filtered(resources.networks(), "", network_name);

        assert_eq!(rows.len(), resources.networks().len());
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
        let lists = LoadedLists::failed("refused");

        assert_eq!(lists.containers, Err(String::from("refused")));
        assert_eq!(lists.images, Err(String::from("refused")));
        assert_eq!(lists.volumes, Err(String::from("refused")));
        assert_eq!(lists.networks, Err(String::from("refused")));
    }
}
