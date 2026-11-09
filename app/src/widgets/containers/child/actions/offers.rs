//! What the user can do with a row (spec Decision 11): the actions each
//! container state and image offers, when Remove is disabled, what removing
//! an image does and the text that tells the user about a failed action.

use otty_containers::{ContainerState, EngineKind, Image, Network, Volume};

use super::super::super::types::ActionRequest;
use super::types::RowAction;
use crate::i18n::{self, Key};

/// The only action of a volume or a network row.
pub(crate) const REMOVE_ONLY: &[RowAction] = &[RowAction::Remove];

/// What a confirmed Remove of an image row does (spec Decision 11). The engines
/// were asked on real runs of Docker 29 and Podman 5; they differ once a
/// container uses the image and this row is its last name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageRemoval {
    /// The image keeps other names, so only this one goes.
    OnlyThisTag,
    /// The last name of an image no container uses: the image goes.
    WholeImage,
    /// Docker, last tag, a container uses the image: Force takes the tag away
    /// and the image stays without one.
    ForceUntags,
    /// Docker, no tag, only stopped containers use the image: Force deletes
    /// the image and they stay.
    ForceKeepsContainers,
    /// Podman, only stopped containers use the image: Force deletes the image
    /// and them, so its switch starts off.
    ForceDeletesContainers,
    /// A running container uses the image and Force would not help.
    Refused,
}

impl ImageRemoval {
    /// Whether the dialog offers the Force switch: the engine refuses without
    /// it and does something with it. Podman with a running container gets no
    /// switch: Force would delete its stopped containers and then fail anyway.
    pub(crate) fn offers_force(self) -> bool {
        matches!(
            self,
            Self::ForceUntags
                | Self::ForceKeepsContainers
                | Self::ForceDeletesContainers
        )
    }

    /// What removing `image` does on an engine of `kind`; `has_running_user`
    /// says whether a running, paused or restarting container uses it.
    pub(crate) fn of(
        kind: EngineKind,
        image: &Image,
        has_running_user: bool,
    ) -> Self {
        if image.has_other_names() {
            return Self::OnlyThisTag;
        }
        if image.containers() == 0 {
            return Self::WholeImage;
        }

        match kind {
            EngineKind::Docker if image.tag().is_some() => Self::ForceUntags,
            _ if has_running_user => Self::Refused,
            EngineKind::Docker => Self::ForceKeepsContainers,
            EngineKind::Podman => Self::ForceDeletesContainers,
        }
    }
}

/// The actions a container in `state` offers, in button order (spec
/// Decision 11).
pub(crate) fn container_actions(state: ContainerState) -> &'static [RowAction] {
    use RowAction::*;

    match state {
        ContainerState::Running => &[Shell, Restart, Stop, Pause, Remove],
        ContainerState::Paused => &[Unpause, Stop, Remove],
        ContainerState::Restarting => &[Stop, Remove],
        ContainerState::Exited | ContainerState::Created => &[Start, Remove],
        ContainerState::Removing => &[],
        _ => &[Remove],
    }
}

/// Whether the engine refuses to remove a container in `state` unless it
/// is stopped first: the remove dialog then offers "Stop it first".
pub(crate) fn needs_stop(state: ContainerState) -> bool {
    matches!(
        state,
        ContainerState::Running
            | ContainerState::Paused
            | ContainerState::Restarting
    )
}

/// The actions an image row offers: Run needs a name and Pull again needs a
/// tag, so a dangling image can only be removed.
pub(crate) fn image_actions(image: &Image) -> &'static [RowAction] {
    if image.is_dangling() {
        &[RowAction::Remove]
    } else if image.tag().is_none() {
        &[RowAction::Run, RowAction::Remove]
    } else {
        &[RowAction::Run, RowAction::PullAgain, RowAction::Remove]
    }
}

/// Why Remove of `volume` is disabled: the engine keeps a volume a
/// container uses. `None` when it can be removed.
pub(crate) fn volume_remove_hint(volume: &Volume) -> Option<String> {
    if volume.containers().is_empty() {
        return None;
    }

    Some(i18n::containers_volume_in_use(
        &volume.containers().join(", "),
    ))
}

/// Why Remove of `network` is disabled: the engine refuses the built-in
/// networks (`bridge`, `host`, `none`) and one a container is connected
/// to. `None` when it can be removed.
pub(crate) fn network_remove_hint(network: &Network) -> Option<String> {
    if network.is_built_in() {
        return Some(i18n::t(Key::ContainersBuiltInNetworkHint).to_string());
    }
    if !network.containers().is_empty() {
        return Some(i18n::t(Key::ContainersNetworkInUseHint).to_string());
    }

    None
}

/// The text that tells the user about a failed `request`: the action, the
/// row and the engine's `message`.
pub(crate) fn failure_text(request: &ActionRequest, message: &str) -> String {
    i18n::containers_action_failed(
        request.action().failure_key(),
        request.name(),
        message,
    )
}

#[cfg(test)]
mod tests {
    use otty_containers::{ContainerState, EngineKind, fixtures};

    use super::*;
    use crate::i18n;
    use crate::widgets::containers::types::EngineAction;

    fn request(action: EngineAction) -> ActionRequest {
        ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            action,
            String::from("otty-fx-running"),
        )
    }

    #[test]
    fn each_container_state_offers_its_actions() {
        use RowAction::*;

        let offered = |state| container_actions(state).to_vec();

        assert_eq!(
            offered(ContainerState::Running),
            vec![Shell, Restart, Stop, Pause, Remove]
        );
        assert_eq!(
            offered(ContainerState::Paused),
            vec![Unpause, Stop, Remove]
        );
        assert_eq!(offered(ContainerState::Restarting), vec![Stop, Remove]);
        assert_eq!(offered(ContainerState::Exited), vec![Start, Remove]);
        assert_eq!(offered(ContainerState::Created), vec![Start, Remove]);
        assert_eq!(offered(ContainerState::Dead), vec![Remove]);
        assert_eq!(offered(ContainerState::Unknown), vec![Remove]);
        assert!(offered(ContainerState::Removing).is_empty());
    }

    /// One row of an image with these names, `containers` of them using it.
    fn image_row(
        names: &[&str],
        digests: &[&str],
        containers: usize,
        row: usize,
    ) -> Image {
        let users: Vec<_> = (0..containers)
            .map(|index| {
                fixtures::container_of_image(
                    &format!("user-{index}"),
                    "exited",
                    "app:1",
                    "sha256:app",
                )
            })
            .collect();

        fixtures::image_rows("sha256:app", names, digests, &users)[row].clone()
    }

    #[test]
    fn removing_a_name_that_is_not_the_last_is_only_that_name_on_any_engine() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            for containers in [0, 1] {
                for has_running_user in [false, containers > 0] {
                    let tag = image_row(&["a:1", "a:2"], &[], containers, 0);
                    let digest =
                        image_row(&["a:1"], &["reg/a@sha256:1"], containers, 1);

                    for row in [tag, digest] {
                        let removal =
                            ImageRemoval::of(kind, &row, has_running_user);

                        assert_eq!(removal, ImageRemoval::OnlyThisTag);
                        assert!(!removal.offers_force());
                    }
                }
            }
        }
    }

    #[test]
    fn the_last_name_of_an_unused_image_takes_the_image_on_any_engine() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            for (names, digests) in [
                (&["a:1"][..], &[][..]),
                (&[][..], &[][..]),
                (&[][..], &["reg/a@sha256:1"][..]),
            ] {
                let row = image_row(names, digests, 0, 0);

                let removal = ImageRemoval::of(kind, &row, false);

                assert_eq!(removal, ImageRemoval::WholeImage);
                assert!(!removal.offers_force());
            }
        }
    }

    #[test]
    fn on_docker_force_leaves_a_used_image_without_its_last_tag() {
        let row = image_row(&["a:1"], &[], 1, 0);

        for has_running_user in [false, true] {
            let removal =
                ImageRemoval::of(EngineKind::Docker, &row, has_running_user);

            assert_eq!(removal, ImageRemoval::ForceUntags);
            assert!(removal.offers_force());
        }
    }

    #[test]
    fn on_docker_force_deletes_an_untagged_image_stopped_containers_use() {
        for digests in [&[][..], &["reg/a@sha256:1"][..]] {
            let row = image_row(&[], digests, 1, 0);

            let removal = ImageRemoval::of(EngineKind::Docker, &row, false);

            assert_eq!(removal, ImageRemoval::ForceKeepsContainers);
            assert!(removal.offers_force());
        }
    }

    #[test]
    fn on_docker_force_does_not_help_an_untagged_image_a_running_container_uses()
     {
        for digests in [&[][..], &["reg/a@sha256:1"][..]] {
            let row = image_row(&[], digests, 1, 0);

            let removal = ImageRemoval::of(EngineKind::Docker, &row, true);

            assert_eq!(removal, ImageRemoval::Refused);
            assert!(!removal.offers_force());
        }
    }

    #[test]
    fn on_podman_force_also_deletes_the_stopped_containers() {
        for names in [&["a:1"][..], &[][..]] {
            let row = image_row(names, &[], 1, 0);

            let removal = ImageRemoval::of(EngineKind::Podman, &row, false);

            assert_eq!(removal, ImageRemoval::ForceDeletesContainers);
            assert!(removal.offers_force());
        }
    }

    #[test]
    fn on_podman_force_is_never_offered_while_a_container_runs() {
        for names in [&["a:1"][..], &[][..]] {
            let row = image_row(names, &[], 1, 0);

            let removal = ImageRemoval::of(EngineKind::Podman, &row, true);

            assert_eq!(removal, ImageRemoval::Refused);
            assert!(!removal.offers_force());
        }
    }

    #[test]
    fn an_image_known_only_by_its_digest_cannot_be_pulled_again() {
        let row = image_row(&[], &["reg/a@sha256:1"], 0, 0);

        assert_eq!(image_actions(&row), &[RowAction::Run, RowAction::Remove]);
    }

    #[test]
    fn a_running_paused_or_restarting_container_is_stopped_before_removal() {
        assert!(needs_stop(ContainerState::Running));
        assert!(needs_stop(ContainerState::Paused));
        assert!(needs_stop(ContainerState::Restarting));
        assert!(!needs_stop(ContainerState::Exited));
        assert!(!needs_stop(ContainerState::Created));
        assert!(!needs_stop(ContainerState::Dead));
    }

    #[test]
    fn a_failure_names_the_action_the_row_and_the_engine_message() {
        let stop = request(EngineAction::StopContainer(String::from("c1")));
        let prune = request(EngineAction::PruneContainers);

        let stop_text = failure_text(&stop, "it is restarting");
        let prune_text = failure_text(&prune, "engine is busy");

        assert_eq!(
            stop_text,
            i18n::containers_action_failed(
                Key::TplContainersStopFailed,
                "otty-fx-running",
                "it is restarting"
            )
        );
        assert!(stop_text.contains("otty-fx-running"), "{stop_text}");
        assert!(stop_text.contains("it is restarting"), "{stop_text}");
        assert!(prune_text.contains("engine is busy"), "{prune_text}");
    }

    #[test]
    fn a_tagged_image_can_be_run_and_pulled_again_and_a_dangling_one_only_removed()
     {
        let resources = fixtures::resources(EngineKind::Docker);
        let dangling = resources
            .images()
            .iter()
            .find(|image| image.is_dangling())
            .expect("fixture has a dangling image");
        let tagged = resources
            .images()
            .iter()
            .find(|image| !image.is_dangling())
            .expect("fixture has a tagged image");

        assert_eq!(
            image_actions(tagged),
            &[RowAction::Run, RowAction::PullAgain, RowAction::Remove]
        );
        assert_eq!(image_actions(dangling), &[RowAction::Remove]);
        assert!(!RowAction::PullAgain.is_danger());
        assert!(!RowAction::Run.is_danger());
    }

    #[test]
    fn a_volume_a_container_uses_cannot_be_removed() {
        let resources = fixtures::resources(EngineKind::Docker);
        let used = resources
            .volumes()
            .iter()
            .find(|volume| !volume.containers().is_empty())
            .expect("fixture has a used volume");
        let names = used.containers().join(", ");

        let hint = volume_remove_hint(used);

        assert_eq!(hint, Some(i18n::containers_volume_in_use(&names)));
        assert!(hint.is_some_and(|hint| hint.contains(&names)));
    }

    #[test]
    fn an_unused_volume_or_network_can_be_removed() {
        let idle = fixtures::idle_resources(EngineKind::Docker);
        let user_network = idle
            .networks()
            .iter()
            .find(|network| network.name() == "otty-fx-net")
            .expect("fixture network");

        for volume in idle.volumes() {
            assert_eq!(volume_remove_hint(volume), None);
        }
        assert_eq!(network_remove_hint(user_network), None);
    }

    #[test]
    fn built_in_and_connected_networks_cannot_be_removed() {
        let resources = fixtures::resources(EngineKind::Docker);
        let network = |name: &str| {
            resources
                .networks()
                .iter()
                .find(|network| network.name() == name)
                .expect("fixture network")
        };

        assert_eq!(
            network_remove_hint(network("bridge")),
            Some(i18n::t(Key::ContainersBuiltInNetworkHint).to_string())
        );
        assert_eq!(
            network_remove_hint(network("host")),
            Some(i18n::t(Key::ContainersBuiltInNetworkHint).to_string())
        );
        assert_eq!(
            network_remove_hint(network("none")),
            Some(i18n::t(Key::ContainersBuiltInNetworkHint).to_string())
        );
        assert_eq!(
            network_remove_hint(network("otty-fx-net")),
            Some(i18n::t(Key::ContainersNetworkInUseHint).to_string())
        );
    }
}
