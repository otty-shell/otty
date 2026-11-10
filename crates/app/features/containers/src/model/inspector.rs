use otty_containers::{Container, Image, Network, Volume};
use otty_ui_kit::components::status_badge::StatusTone;

use super::super::resource_rows::{
    container_status, image_status, network_status, volume_status,
};
use super::super::state::ContainersState;
use super::super::types::{ResourceKind, image_name};

/// Header of the inspector: the selected row's name and its status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorViewModel {
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) tone: StatusTone,
}

impl InspectorViewModel {
    /// Build the inspector header of the selected row of the open list;
    /// `None` when nothing is selected.
    pub(in super::super) fn from_state(
        state: &ContainersState,
    ) -> Option<Self> {
        let place = state.place()?;
        let engine = state.open_engine()?;

        let (title, (status, tone)) = match place.resource() {
            ResourceKind::Containers => {
                let container = engine.containers().selected(Container::id)?;
                (container.name(), container_status(container))
            },
            ResourceKind::Images => {
                let image = engine.images().selected(Image::row_key)?;
                (image_name(image), image_status(image))
            },
            ResourceKind::Volumes => {
                let volume = engine.volumes().selected(Volume::name)?;
                (volume.name(), volume_status(volume))
            },
            ResourceKind::Networks => {
                let network = engine.networks().selected(Network::id)?;
                (network.name(), network_status(network))
            },
        };

        Some(Self {
            title: title.to_string(),
            status,
            tone,
        })
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{Container, EngineKind, fixtures};
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::super::test_support::{
        build, docker, found, lists, load, open, two_tags_of_one_image,
    };
    use super::*;
    use crate::event::ContainersIntent;
    use crate::reducer::reduce;
    use crate::resource_rows::{self};
    use crate::state::ContainersState;
    use crate::types::ResourceKind;

    fn reload_images(state: &mut ContainersState, names: &[&str]) {
        let mut loaded = lists(EngineKind::Docker);
        loaded.images = Ok(fixtures::image_rows("sha256:app", names, &[], &[]));
        load(state, &docker(), loaded);
    }

    #[test]
    fn the_inspector_names_the_selected_tag() {
        let mut state = two_tags_of_one_image();

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:2"))
        );
    }

    #[test]
    fn a_reload_keeps_the_same_tag_selected() {
        let mut state = two_tags_of_one_image();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        reload_images(&mut state, &["app:1", "app:2", "app:3"]);

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:2"))
        );
    }

    #[test]
    fn a_reload_without_the_selected_tag_selects_as_for_any_removed_row() {
        let mut state = two_tags_of_one_image();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        reload_images(&mut state, &["app:1"]);

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:1"))
        );
    }

    #[test]
    fn the_inspector_shows_the_selected_row_of_each_resource() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let resources = fixtures::resources(EngineKind::Docker);
        let first: &Container = &resources.containers()[0];

        let containers = build(&state).inspector;
        open(&mut state, ResourceKind::Images);
        let images = build(&state).inspector;
        open(&mut state, ResourceKind::Volumes);
        let volumes = build(&state).inspector;
        open(&mut state, ResourceKind::Networks);
        let networks = build(&state).inspector;

        let header = |title: &str, (status, tone): (String, StatusTone)| {
            Some(InspectorViewModel {
                title: title.to_string(),
                status,
                tone,
            })
        };
        assert_eq!(
            containers,
            header(first.name(), resource_rows::container_status(first))
        );
        assert_eq!(
            images,
            header(
                "<none>:<none>",
                resource_rows::image_status(&resources.images()[0])
            )
        );
        assert_eq!(
            volumes,
            header(
                resources.volumes()[0].name(),
                resource_rows::volume_status(&resources.volumes()[0])
            )
        );
        assert_eq!(
            networks,
            header(
                "bridge",
                resource_rows::network_status(&resources.networks()[0])
            )
        );
    }
}
