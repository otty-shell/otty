//! An image as OTTY shows it, mapped from the engine's image list.

#[cfg(feature = "client")]
use bollard::models::ImageSummary;

#[cfg(feature = "client")]
use crate::container::Container;

/// The text an engine puts in a name it does not have: older Docker sends
/// `<none>:<none>` and `<none>@<none>`.
#[cfg(feature = "client")]
const NO_NAME: &str = "<none>";
/// The tag a name without one means.
#[cfg(feature = "client")]
const DEFAULT_TAG: &str = "latest";

/// One row of the engine's image list, as `docker images` shows it (spec
/// Decision 10): one per `repository:tag`. Rows of one image share its id, its
/// size and its containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    id: String,
    repository: Option<String>,
    tag: Option<String>,
    reference: Option<String>,
    row_key: String,
    has_other_names: bool,
    size: u64,
    created: i64,
    containers: usize,
}

impl Image {
    /// The full image id, for example `sha256:a7dbc319…`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The repository of this row, for example `busybox` (Docker) or
    /// `docker.io/library/busybox` (Podman). `None` for a dangling image.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let rows = fixtures::image_rows("sha256:one", &["busybox:1.37"], &[], &[]);
    /// let dangling = fixtures::image_rows("sha256:two", &[], &[], &[]);
    ///
    /// assert_eq!(rows[0].repository(), Some("busybox"));
    /// assert_eq!(dangling[0].repository(), None);
    /// # }
    /// ```
    pub fn repository(&self) -> Option<&str> {
        self.repository.as_deref()
    }

    /// The tag of this row, for example `1.37`. `None` for a dangling image
    /// and for one known only by its digest.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let tagged =
    ///     fixtures::image_rows("sha256:one", &["busybox:1.37"], &[], &[]);
    /// let digest_only =
    ///     fixtures::image_rows("sha256:two", &[], &["reg/app@sha256:aa"], &[]);
    ///
    /// assert_eq!(tagged[0].tag(), Some("1.37"));
    /// assert_eq!(digest_only[0].tag(), None);
    /// # }
    /// ```
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// The name this row stands for: `repository:tag`, or
    /// `repository@sha256:…` for an image known only by its digest. `None`
    /// for a dangling image.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let tagged =
    ///     fixtures::image_rows("sha256:one", &["busybox:1.37"], &[], &[]);
    /// let digest_only =
    ///     fixtures::image_rows("sha256:two", &[], &["reg/app@sha256:aa"], &[]);
    /// let dangling = fixtures::image_rows("sha256:three", &[], &[], &[]);
    ///
    /// assert_eq!(tagged[0].reference(), Some("busybox:1.37"));
    /// assert_eq!(digest_only[0].reference(), Some("reg/app@sha256:aa"));
    /// assert_eq!(dangling[0].reference(), None);
    /// # }
    /// ```
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }

    /// What tells this row from every other row of the list, also from the
    /// other rows of its image.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let rows =
    ///     fixtures::image_rows("sha256:one", &["app:1", "app:2"], &[], &[]);
    ///
    /// assert_eq!(rows[0].id(), rows[1].id());
    /// assert_ne!(rows[0].row_key(), rows[1].row_key());
    /// # }
    /// ```
    pub fn row_key(&self) -> &str {
        &self.row_key
    }

    /// Whether the image keeps another name once this row's name goes. Then
    /// a remove takes away only this name.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let two_tags =
    ///     fixtures::image_rows("sha256:one", &["app:1", "app:2"], &[], &[]);
    /// let one_tag = fixtures::image_rows("sha256:two", &["app:1"], &[], &[]);
    ///
    /// assert!(two_tags[0].has_other_names());
    /// assert!(!one_tag[0].has_other_names());
    /// # }
    /// ```
    pub fn has_other_names(&self) -> bool {
        self.has_other_names
    }

    /// What a remove call of this row names, so only this row goes: its
    /// reference when it has a tag or the image has other names, else the
    /// image id.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::fixtures;
    ///
    /// let tagged = fixtures::image_rows("sha256:one", &["app:1"], &[], &[]);
    /// let dangling = fixtures::image_rows("sha256:two", &[], &[], &[]);
    ///
    /// assert_eq!(tagged[0].remove_target(), "app:1");
    /// assert_eq!(dangling[0].remove_target(), "sha256:two");
    /// # }
    /// ```
    pub fn remove_target(&self) -> &str {
        match self.reference.as_deref() {
            Some(reference) if self.tag.is_some() || self.has_other_names => {
                reference
            },
            _ => &self.id,
        }
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
        self.repository.is_none()
    }
}

#[cfg(feature = "client")]
impl Image {
    /// Map one entry of the engine's image list to its rows, as `docker
    /// images` prints them. The container count comes from `containers`:
    /// Docker before 29 sends `-1` instead of a count.
    pub(crate) fn rows_from_summary(
        summary: &ImageSummary,
        containers: &[Container],
    ) -> Vec<Self> {
        let users = containers
            .iter()
            .filter(|container| container.image_id() == summary.id)
            .count();
        let names = row_names(summary);

        if names.is_empty() {
            return vec![Self::row(summary, users, None, false)];
        }

        names
            .iter()
            .map(|name| {
                let has_other_names =
                    names.len() > 1 || has_digest_elsewhere(summary, name);

                Self::row(summary, users, Some(name), has_other_names)
            })
            .collect()
    }

    /// The row of `summary` under `name`; a dangling image has no name.
    fn row(
        summary: &ImageSummary,
        users: usize,
        name: Option<&RowName<'_>>,
        has_other_names: bool,
    ) -> Self {
        let reference = name.map(|name| name.reference.to_string());
        let row_key = match &reference {
            Some(reference) => format!("{} {reference}", summary.id),
            None => summary.id.clone(),
        };

        Self {
            id: summary.id.clone(),
            repository: name.map(|name| name.repository.to_string()),
            tag: name.and_then(|name| name.tag).map(str::to_string),
            reference,
            row_key,
            has_other_names,
            size: u64::try_from(summary.size).unwrap_or_default(),
            created: summary.created,
            containers: users,
        }
    }
}

/// The name of one row: a `repository:tag`, or a `repository@digest` when the
/// repository has no tag.
#[cfg(feature = "client")]
struct RowName<'a> {
    repository: &'a str,
    tag: Option<&'a str>,
    reference: &'a str,
}

/// The names that get a row, as the `REPOSITORY` and `TAG` columns of `docker
/// images` print them: each tag, then each repository of `RepoDigests` that
/// has no tag. Docker 29 also lists digests in `RepoTags`; `docker images`
/// prints no row for those, but such a name is all that is left of an image
/// after its tags go, so then it does get a row.
#[cfg(feature = "client")]
fn row_names(summary: &ImageSummary) -> Vec<RowName<'_>> {
    let mut names: Vec<RowName<'_>> = summary
        .repo_tags
        .iter()
        .filter(|name| is_tag(name))
        .map(|name| tag_name(name))
        .collect();

    add_digest_names(&mut names, &summary.repo_digests);
    if names.is_empty() {
        add_digest_names(&mut names, &summary.repo_tags);
    }

    names
}

/// Add a name for each repository in `references` that has none yet.
#[cfg(feature = "client")]
fn add_digest_names<'a>(
    names: &mut Vec<RowName<'a>>,
    references: &'a [String],
) {
    for reference in references {
        let Some(repository) = digest_repository(reference) else {
            continue;
        };
        if names.iter().all(|name| name.repository != repository) {
            names.push(RowName {
                repository,
                tag: None,
                reference,
            });
        }
    }
}

/// Whether the image has a digest in another repository than `name`, which
/// stays after `name` goes. The engines delete the digests of the name's own
/// repository along with its last tag.
#[cfg(feature = "client")]
fn has_digest_elsewhere(summary: &ImageSummary, name: &RowName<'_>) -> bool {
    summary
        .repo_tags
        .iter()
        .chain(&summary.repo_digests)
        .filter_map(|reference| digest_repository(reference))
        .any(|repository| repository != name.repository)
}

/// Whether `name` is a real `repository:tag`.
#[cfg(feature = "client")]
fn is_tag(name: &str) -> bool {
    !name.contains(NO_NAME) && !name.contains('@')
}

/// Read a `repository:tag`. A colon before the last slash belongs to a registry
/// port; a name without a tag means `latest`.
#[cfg(feature = "client")]
fn tag_name(reference: &str) -> RowName<'_> {
    let (repository, tag) = reference
        .rsplit_once(':')
        .filter(|(_, tag)| !tag.contains('/'))
        .unwrap_or((reference, DEFAULT_TAG));

    RowName {
        repository,
        tag: Some(tag),
        reference,
    }
}

/// The repository of a `repository@digest`; `None` for anything else.
#[cfg(feature = "client")]
fn digest_repository(reference: &str) -> Option<&str> {
    if reference.contains(NO_NAME) {
        return None;
    }

    reference.split_once('@').map(|(repository, _)| repository)
}

#[cfg(all(test, feature = "client"))]
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
            .flat_map(|summary| Image::rows_from_summary(summary, &containers))
            .collect()
    }

    fn named<'a>(images: &'a [Image], reference: &str) -> &'a Image {
        images
            .iter()
            .find(|image| image.reference() == Some(reference))
            .unwrap_or_else(|| panic!("no image {reference}"))
    }

    /// The rows of an image with these lists, as the engine would send them.
    fn rows(repo_tags: &[&str], repo_digests: &[&str]) -> Vec<Image> {
        let summary: ImageSummary = serde_json::from_value(serde_json::json!({
            "Id": "sha256:one",
            "ParentId": "",
            "RepoTags": repo_tags,
            "RepoDigests": repo_digests,
            "Created": 1,
            "Size": 10,
            "SharedSize": -1,
            "Labels": null,
            "Containers": -1,
        }))
        .expect("image summary");

        Image::rows_from_summary(&summary, &[])
    }

    /// What `docker images` prints in its two name columns.
    fn columns(rows: &[Image]) -> Vec<(Option<&str>, Option<&str>)> {
        rows.iter()
            .map(|row| (row.repository(), row.tag()))
            .collect()
    }

    #[test]
    fn docker_image_is_mapped_with_its_container_count() {
        let images = images(DOCKER_IMAGES, DOCKER_CONTAINERS);

        let busybox = named(&images, "busybox:1.37");

        assert_eq!(
            busybox.id(),
            "sha256:a7dbc319b1438115a207e11930dc4bb3e643dcb3ec0bdd694bb36cd8fad289ec"
        );
        assert_eq!(busybox.repository(), Some("busybox"));
        assert_eq!(busybox.tag(), Some("1.37"));
        assert_eq!(busybox.size(), 6_688_167);
        assert_eq!(busybox.created(), 1_727_386_302);
        assert_eq!(busybox.containers(), 6);
        assert!(!busybox.is_dangling());
    }

    #[test]
    fn podman_image_is_mapped_with_its_container_count() {
        let images = images(PODMAN_IMAGES, PODMAN_CONTAINERS);

        let busybox = named(&images, "docker.io/library/busybox:1.37");

        assert_eq!(busybox.repository(), Some("docker.io/library/busybox"));
        assert_eq!(busybox.tag(), Some("1.37"));
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

            assert_eq!(named(&images, tag).containers(), 0);
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
            assert_eq!(dangling[0].repository(), None);
            assert_eq!(dangling[0].tag(), None);
            assert_eq!(dangling[0].reference(), None);
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

        let rows = Image::rows_from_summary(&summary, &containers);

        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_dangling());
        assert_eq!(rows[0].containers(), 1);
        assert_eq!(rows[0].size(), 0);
    }

    #[test]
    fn each_tag_is_its_own_row_with_the_same_image_facts() {
        let rows = rows(&["app:1", "app:2", "mirror/app:1"], &[]);

        assert_eq!(
            columns(&rows),
            vec![
                (Some("app"), Some("1")),
                (Some("app"), Some("2")),
                (Some("mirror/app"), Some("1")),
            ]
        );
        for row in &rows {
            assert_eq!(row.id(), "sha256:one");
            assert_eq!(row.size(), 10);
            assert_eq!(row.created(), 1);
        }
    }

    #[test]
    fn rows_of_one_image_have_different_keys() {
        let rows = rows(&["app:1", "app:2"], &[]);

        assert_ne!(rows[0].row_key(), rows[1].row_key());
    }

    #[test]
    fn a_registry_port_is_part_of_the_repository() {
        let rows = rows(&["localhost:5000/team/app:v2"], &[]);

        assert_eq!(
            columns(&rows),
            vec![(Some("localhost:5000/team/app"), Some("v2"))]
        );
        assert_eq!(rows[0].reference(), Some("localhost:5000/team/app:v2"));
    }

    #[test]
    fn a_digest_of_a_repository_with_a_tag_has_no_row() {
        // Docker 29 also puts the digest in `RepoTags`.
        let rows = rows(&["app:1", "app@sha256:aa"], &["app@sha256:aa"]);

        assert_eq!(columns(&rows), vec![(Some("app"), Some("1"))]);
        assert!(!rows[0].has_other_names());
    }

    #[test]
    fn an_image_known_only_by_its_digest_is_its_repository_without_a_tag() {
        let rows = rows(&["reg/app@sha256:aa"], &["reg/app@sha256:aa"]);

        assert_eq!(columns(&rows), vec![(Some("reg/app"), None)]);
        assert_eq!(rows[0].reference(), Some("reg/app@sha256:aa"));
        assert!(!rows[0].is_dangling());
    }

    #[test]
    fn a_digest_in_the_repo_tags_alone_still_gives_a_row() {
        let rows = rows(&["reg/app@sha256:aa"], &[]);

        assert_eq!(columns(&rows), vec![(Some("reg/app"), None)]);
    }

    #[test]
    fn a_digest_of_another_repository_is_a_row_as_in_docker_images() {
        let rows = rows(&["app:1"], &["reg/app@sha256:aa"]);

        assert_eq!(
            columns(&rows),
            vec![(Some("app"), Some("1")), (Some("reg/app"), None)]
        );
    }

    #[test]
    fn several_digests_of_one_repository_are_one_row() {
        let rows = rows(&[], &["reg/app@sha256:aa", "reg/app@sha256:bb"]);

        assert_eq!(columns(&rows), vec![(Some("reg/app"), None)]);
    }

    #[test]
    fn a_digest_only_in_the_repo_tags_has_no_row_beside_a_tag_but_counts_as_a_name()
     {
        // Docker 29 keeps this name; `docker images` does not print it, and
        // removing the tag leaves the image under it.
        let rows = rows(&["app:1", "mirror/app@sha256:aa"], &["app@sha256:aa"]);

        assert_eq!(columns(&rows), vec![(Some("app"), Some("1"))]);
        assert!(rows[0].has_other_names());
    }

    #[test]
    fn a_row_has_other_names_only_when_something_stays() {
        assert!(!rows(&["app:1"], &[])[0].has_other_names());
        assert!(rows(&["app:1", "app:2"], &[])[0].has_other_names());
        assert!(rows(&["app:1"], &["reg/app@sha256:aa"])[0].has_other_names());
        assert!(!rows(&[], &[])[0].has_other_names());
    }

    #[test]
    fn a_tagged_row_is_removed_by_its_reference() {
        assert_eq!(rows(&["app:1"], &[])[0].remove_target(), "app:1");
        assert_eq!(rows(&["app:1", "app:2"], &[])[1].remove_target(), "app:2");
    }

    #[test]
    fn a_row_without_a_tag_is_removed_by_id() {
        assert_eq!(rows(&[], &[])[0].remove_target(), "sha256:one");
        assert_eq!(
            rows(&[], &["reg/app@sha256:aa"])[0].remove_target(),
            "sha256:one"
        );
    }

    #[test]
    fn a_digest_row_beside_other_names_is_removed_by_its_digest() {
        let rows = rows(&["app:1"], &["reg/app@sha256:aa"]);

        assert_eq!(rows[1].remove_target(), "reg/app@sha256:aa");
    }
}
