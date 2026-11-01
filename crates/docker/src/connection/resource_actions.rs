//! Image, volume and network actions on an engine: remove and prune
//! images, create, remove and prune volumes and networks (spec Decision 11).

use std::collections::HashMap;

use bollard::models::{NetworkCreateRequest, VolumeCreateRequest};
use bollard::query_parameters::{
    PruneImagesOptions, PruneNetworksOptions, PruneVolumesOptions,
    RemoveImageOptions, RemoveVolumeOptions,
};

use super::{Engine, EngineError, parse_api_version};
use crate::engine::{EngineInfo, EngineKind};

/// The first API version (Docker 23) whose volume prune removes only
/// anonymous volumes unless the `all` filter is set.
const PRUNE_ALL_VOLUMES_SINCE: (usize, usize) = (1, 42);

impl Engine {
    /// Remove an image by id. The engine refuses an image a container uses
    /// unless `force` is set; it always refuses one a running container
    /// uses.
    pub async fn remove_image(
        &self,
        id: &str,
        force: bool,
    ) -> Result<(), EngineError> {
        let options = RemoveImageOptions {
            force,
            ..RemoveImageOptions::default()
        };

        self.client.remove_image(id, Some(options), None).await?;

        Ok(())
    }

    /// Remove every dangling image (no tag) that no container uses.
    pub async fn prune_images(&self) -> Result<(), EngineError> {
        let options = PruneImagesOptions {
            filters: Some(filter("dangling", "true")),
        };

        self.client.prune_images(Some(options)).await?;

        Ok(())
    }

    /// Create a volume. A blank `name` lets the engine pick one.
    pub async fn create_volume(
        &self,
        name: &str,
        driver: &str,
    ) -> Result<(), EngineError> {
        let name = name.trim();
        let request = VolumeCreateRequest {
            name: (!name.is_empty()).then(|| name.to_string()),
            driver: Some(driver.trim().to_string()),
            ..VolumeCreateRequest::default()
        };

        self.client.create_volume(request).await?;

        Ok(())
    }

    /// Remove a volume by name. The engine refuses one a container uses.
    pub async fn remove_volume(&self, name: &str) -> Result<(), EngineError> {
        self.client
            .remove_volume(name, None::<RemoveVolumeOptions>)
            .await?;

        Ok(())
    }

    /// Remove every volume no container uses, named ones included, on
    /// Docker and on Podman alike.
    pub async fn prune_volumes(&self) -> Result<(), EngineError> {
        let options = volume_prune_options(self.info());

        self.client.prune_volumes(options).await?;

        Ok(())
    }

    /// Create a network.
    pub async fn create_network(
        &self,
        name: &str,
        driver: &str,
    ) -> Result<(), EngineError> {
        let request = NetworkCreateRequest {
            name: name.trim().to_string(),
            driver: Some(driver.trim().to_string()),
            ..NetworkCreateRequest::default()
        };

        self.client.create_network(request).await?;

        Ok(())
    }

    /// Remove a network by id. The engine refuses a built-in network and
    /// one a container is connected to.
    pub async fn remove_network(&self, id: &str) -> Result<(), EngineError> {
        self.client.remove_network(id).await?;

        Ok(())
    }

    /// Remove every user network no container is connected to.
    pub async fn prune_networks(&self) -> Result<(), EngineError> {
        self.client
            .prune_networks(None::<PruneNetworksOptions>)
            .await?;

        Ok(())
    }
}

/// The volume prune options that remove every unused volume. Docker 23
/// and later (API 1.42) remove only anonymous volumes unless `all=true` is
/// set; older Docker and Podman remove every unused volume without it (and
/// may not know the filter).
fn volume_prune_options(info: &EngineInfo) -> Option<PruneVolumesOptions> {
    if info.kind() != EngineKind::Docker {
        return None;
    }
    let version = info.api_version().and_then(parse_api_version)?;
    let since = PRUNE_ALL_VOLUMES_SINCE;
    if (version.major_version, version.minor_version) < since {
        return None;
    }

    Some(PruneVolumesOptions {
        filters: Some(filter("all", "true")),
    })
}

/// One engine filter: `name` with one `value`.
fn filter(name: &str, value: &str) -> HashMap<String, Vec<String>> {
    HashMap::from([(name.to_string(), vec![value.to_string()])])
}

#[cfg(test)]
mod tests {
    use bollard::models::SystemVersion;

    use super::*;
    use crate::address::EngineAddress;
    use crate::engine::EngineInfo;
    use crate::test_support::FakeEngine;

    const NO_CONTENT: &str = "204 No Content";

    fn engine(fake: &FakeEngine) -> Engine {
        Engine::connect(fake.info()).expect("engine")
    }

    fn answer(status: &'static str, body: &str) -> Vec<(&'static str, String)> {
        vec![(status, body.to_string())]
    }

    /// The method and the path of a request line, without the query.
    fn method_and_path(line: &str) -> (String, String) {
        let mut parts = line.split(' ');
        let method = parts.next().unwrap_or_default().to_string();
        let target = parts.next().unwrap_or_default();
        let path = target.split('?').next().unwrap_or_default().to_string();

        (method, path)
    }

    fn info(version: &str) -> EngineInfo {
        let answer: SystemVersion =
            serde_json::from_str(version).expect("version answer");

        EngineInfo::from_version(
            &answer,
            EngineAddress::Tcp(String::from("127.0.0.1:1")),
        )
    }

    #[tokio::test]
    async fn remove_image_deletes_it_and_forces_only_when_asked() {
        for force in [false, true] {
            let fake = FakeEngine::start(answer(
                "200 OK",
                r#"[{"Deleted":"sha256:abc"}]"#,
            ));

            engine(&fake)
                .remove_image("sha256:abc", force)
                .await
                .expect("image removed");

            let requests = fake.requests();
            let (method, path) = method_and_path(&requests[0].line);
            assert_eq!(method, "DELETE");
            assert!(path.ends_with("/images/sha256:abc"), "{path}");
            assert_eq!(requests[0].line.contains("force=true"), force);
        }
    }

    #[tokio::test]
    async fn prune_images_removes_only_dangling_ones() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"ImagesDeleted":[],"SpaceReclaimed":0}"#,
        ));

        engine(&fake).prune_images().await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/images/prune"), "{path}");
        assert!(
            requests[0].line.contains("dangling"),
            "{}",
            requests[0].line
        );
    }

    #[tokio::test]
    async fn create_volume_sends_its_name_and_driver() {
        let fake = FakeEngine::start(answer(
            "201 Created",
            r#"{"Name":"otty-fx-new","Driver":"local","Mountpoint":"/var/lib/docker/volumes/otty-fx-new/_data","Labels":{},"Scope":"local","Options":{}}"#,
        ));

        engine(&fake)
            .create_volume("otty-fx-new", "local")
            .await
            .expect("volume created");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/volumes/create"), "{path}");
        let body: serde_json::Value =
            serde_json::from_str(&requests[0].body).expect("json body");
        assert_eq!(body["Name"], "otty-fx-new");
        assert_eq!(body["Driver"], "local");
    }

    #[tokio::test]
    async fn a_volume_without_a_name_is_named_by_the_engine() {
        let fake = FakeEngine::start(answer(
            "201 Created",
            r#"{"Name":"0123abcd","Driver":"local","Mountpoint":"/x","Labels":{},"Scope":"local","Options":{}}"#,
        ));

        engine(&fake)
            .create_volume("  ", "local")
            .await
            .expect("volume created");

        let requests = fake.requests();
        let body: serde_json::Value =
            serde_json::from_str(&requests[0].body).expect("json body");
        assert!(body["Name"].is_null(), "{body}");
    }

    #[tokio::test]
    async fn remove_volume_deletes_it_by_name() {
        let fake = FakeEngine::start(answer(NO_CONTENT, ""));

        engine(&fake)
            .remove_volume("otty-fx-data")
            .await
            .expect("volume removed");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "DELETE");
        assert!(path.ends_with("/volumes/otty-fx-data"), "{path}");
    }

    #[tokio::test]
    async fn prune_volumes_on_new_docker_asks_for_every_unused_volume() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"VolumesDeleted":[],"SpaceReclaimed":0}"#,
        ));

        engine(&fake).prune_volumes().await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/volumes/prune"), "{path}");
        assert!(requests[0].line.contains("all"), "{}", requests[0].line);
    }

    #[test]
    fn only_docker_23_and_later_need_the_all_filter() {
        let new_docker = info(r#"{"Version":"29.9.1","ApiVersion":"1.44"}"#);
        let docker_23 = info(r#"{"Version":"23.0.0","ApiVersion":"1.42"}"#);
        let old_docker = info(r#"{"Version":"20.10.0","ApiVersion":"1.41"}"#);
        let unknown = info(r#"{"Version":"29.0.0"}"#);
        let podman = info(
            r#"{"ApiVersion":"1.44","Components":[{"Name":"Podman Engine","Version":"5.8.8"}]}"#,
        );

        assert!(volume_prune_options(&new_docker).is_some());
        assert!(volume_prune_options(&docker_23).is_some());
        assert!(volume_prune_options(&old_docker).is_none());
        assert!(volume_prune_options(&unknown).is_none());
        assert!(volume_prune_options(&podman).is_none());
    }

    #[tokio::test]
    async fn create_network_sends_its_name_and_driver() {
        let fake = FakeEngine::start(answer(
            "201 Created",
            r#"{"Id":"9b5579d1a735","Warning":""}"#,
        ));

        engine(&fake)
            .create_network("otty-fx-net2", "bridge")
            .await
            .expect("network created");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/networks/create"), "{path}");
        let body: serde_json::Value =
            serde_json::from_str(&requests[0].body).expect("json body");
        assert_eq!(body["Name"], "otty-fx-net2");
        assert_eq!(body["Driver"], "bridge");
    }

    #[tokio::test]
    async fn remove_network_deletes_it_by_id() {
        let fake = FakeEngine::start(answer(NO_CONTENT, ""));

        engine(&fake)
            .remove_network("9b5579d1a735")
            .await
            .expect("network removed");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "DELETE");
        assert!(path.ends_with("/networks/9b5579d1a735"), "{path}");
    }

    #[tokio::test]
    async fn prune_networks_posts_to_network_prune() {
        let fake =
            FakeEngine::start(answer("200 OK", r#"{"NetworksDeleted":[]}"#));

        engine(&fake).prune_networks().await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/networks/prune"), "{path}");
    }

    #[tokio::test]
    async fn a_refused_removal_carries_the_engines_own_message() {
        let fake = FakeEngine::start(answer(
            "409 Conflict",
            r#"{"message":"volume is in use - [4f1c2b]"}"#,
        ));

        let result = engine(&fake).remove_volume("otty-fx-data").await;

        let Err(error) = result else {
            panic!("expected an error");
        };
        assert_eq!(error.message(), "volume is in use - [4f1c2b]");
        fake.requests();
    }
}
