//! An image as OTTY shows it, mapped from the engine's image list.

use bollard::models::ImageSummary;

use crate::container::Container;

/// One image of an engine (spec Decision 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    id: String,
    tags: Vec<String>,
    size: u64,
    created: i64,
    containers: usize,
}

impl Image {
    /// The full image id, for example `sha256:a7dbc319…`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `repository:tag` names, for example `busybox:1.37` (Docker) or
    /// `docker.io/library/busybox:1.37` (Podman). Empty for a dangling image.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Size on disk in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// When the image was built, in Unix seconds.
    pub fn created(&self) -> i64 {
        self.created
    }

    /// How many containers (running or not) use this image.
    pub fn containers(&self) -> usize {
        self.containers
    }

    /// An image without a name: an old build whose tag moved to a newer one.
    /// Prune dangling images removes these.
    pub fn is_dangling(&self) -> bool {
        self.tags.is_empty()
    }

    /// Map one entry of the engine's image list. The container count comes
    /// from `containers`: Docker before 29 sends `-1` instead of a count.
    pub(crate) fn from_summary(
        summary: &ImageSummary,
        containers: &[Container],
    ) -> Self {
        let tags = summary
            .repo_tags
            .iter()
            .filter(|tag| !tag.contains("<none>"))
            .cloned()
            .collect();
        let users = containers
            .iter()
            .filter(|container| container.image_id() == summary.id)
            .count();

        Self {
            id: summary.id.clone(),
            tags,
            size: u64::try_from(summary.size).unwrap_or_default(),
            created: summary.created,
            containers: users,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        DOCKER_CONTAINERS, PODMAN_CONTAINERS, containers,
    };

    const DOCKER_IMAGES: &str =
        include_str!("../tests/fixtures/docker-images.json");
    const PODMAN_IMAGES: &str =
        include_str!("../tests/fixtures/podman-images.json");

    fn images(images_json: &str, containers_json: &str) -> Vec<Image> {
        let summaries: Vec<ImageSummary> =
            serde_json::from_str(images_json).expect("image list fixture");
        let containers = containers(containers_json);

        summaries
            .iter()
            .map(|summary| Image::from_summary(summary, &containers))
            .collect()
    }

    fn tagged<'a>(images: &'a [Image], tag: &str) -> &'a Image {
        images
            .iter()
            .find(|image| image.tags().iter().any(|t| t == tag))
            .unwrap_or_else(|| panic!("no image {tag}"))
    }

    #[test]
    fn docker_image_is_mapped_with_its_container_count() {
        let images = images(DOCKER_IMAGES, DOCKER_CONTAINERS);

        let busybox = tagged(&images, "busybox:1.37");

        assert_eq!(
            busybox.id(),
            "sha256:a7dbc319b1438115a207e11930dc4bb3e643dcb3ec0bdd694bb36cd8fad289ec"
        );
        assert_eq!(busybox.size(), 6_688_167);
        assert_eq!(busybox.created(), 1_727_386_302);
        assert_eq!(busybox.containers(), 6);
        assert!(!busybox.is_dangling());
    }

    #[test]
    fn podman_image_is_mapped_with_its_container_count() {
        let images = images(PODMAN_IMAGES, PODMAN_CONTAINERS);

        let busybox = tagged(&images, "docker.io/library/busybox:1.37");

        assert_eq!(busybox.size(), 4_670_414);
        assert_eq!(busybox.containers(), 6);
    }

    #[test]
    fn image_no_container_uses_counts_zero() {
        for (images_json, containers_json, tag) in [
            (DOCKER_IMAGES, DOCKER_CONTAINERS, "otty-fx-img:1"),
            (PODMAN_IMAGES, PODMAN_CONTAINERS, "localhost/otty-fx-img:1"),
        ] {
            let images = images(images_json, containers_json);

            assert_eq!(tagged(&images, tag).containers(), 0);
        }
    }

    #[test]
    fn untagged_image_is_dangling() {
        for (images_json, containers_json) in [
            (DOCKER_IMAGES, DOCKER_CONTAINERS),
            (PODMAN_IMAGES, PODMAN_CONTAINERS),
        ] {
            let images = images(images_json, containers_json);

            let dangling: Vec<&Image> =
                images.iter().filter(|image| image.is_dangling()).collect();

            assert_eq!(dangling.len(), 1);
            assert!(dangling[0].tags().is_empty());
            assert_eq!(dangling[0].containers(), 0);
        }
    }

    #[test]
    fn older_docker_none_tag_is_dangling_and_count_comes_from_containers() {
        // Docker before 29 names a dangling image `<none>:<none>` and sends
        // `-1` containers in the list.
        let summary: ImageSummary = serde_json::from_str(
            r#"{"Id":"sha256:old","ParentId":"","RepoTags":["<none>:<none>"],
                "RepoDigests":["<none>@<none>"],"Created":1,"Size":-1,
                "SharedSize":-1,"Labels":null,"Containers":-1}"#,
        )
        .expect("image summary");
        let container: bollard::models::ContainerSummary =
            serde_json::from_str(r#"{"Id":"c1","ImageID":"sha256:old"}"#)
                .expect("container summary");
        let containers = [Container::from_summary(&container)];

        let image = Image::from_summary(&summary, &containers);

        assert!(image.is_dangling());
        assert_eq!(image.containers(), 1);
        assert_eq!(image.size(), 0);
    }
}
