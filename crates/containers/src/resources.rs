//! The four lists of one engine, loaded together.

#[cfg(feature = "client")]
use bollard::models::{
    ContainerSummary, ImageSummary, Network as EngineNetwork,
    Volume as EngineVolume,
};

use crate::container::Container;
use crate::image::Image;
use crate::network::Network;
use crate::volume::Volume;

/// All four lists of one engine, loaded together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    containers: Vec<Container>,
    images: Vec<Image>,
    volumes: Vec<Volume>,
    networks: Vec<Network>,
}

impl Resources {
    /// Every container, running or not, in the engine's order (newest first).
    pub fn containers(&self) -> &[Container] {
        &self.containers
    }

    /// Every top-level image, one row per `repository:tag` as `docker images`
    /// shows them, dangling ones included.
    pub fn images(&self) -> &[Image] {
        &self.images
    }

    /// Every volume.
    pub fn volumes(&self) -> &[Volume] {
        &self.volumes
    }

    /// Every network, built-in ones included.
    pub fn networks(&self) -> &[Network] {
        &self.networks
    }
}

#[cfg(feature = "client")]
impl Resources {
    /// Map the four list answers of one engine; "used by" comes from the
    /// container list of the same answers.
    pub(crate) fn from_answers(
        container_list: &[ContainerSummary],
        image_list: &[ImageSummary],
        volume_list: &[EngineVolume],
        network_list: &[EngineNetwork],
    ) -> Self {
        let containers: Vec<Container> =
            container_list.iter().map(Container::from_summary).collect();

        let images = image_list
            .iter()
            .flat_map(|image| Image::rows_from_summary(image, &containers))
            .collect();
        let volumes = volume_list
            .iter()
            .map(|volume| Volume::from_volume(volume, &containers))
            .collect();
        let networks = network_list
            .iter()
            .map(|network| Network::from_network(network, &containers))
            .collect();

        Self {
            containers,
            images,
            volumes,
            networks,
        }
    }
}
