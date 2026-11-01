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

    /// Remove the volumes no container uses. Only the anonymous ones go,
    /// unless `include_named` is set; then every unused volume goes, like
    /// `docker volume prune -a`.
    ///
    /// Docker 23 and later (API 1.42) can prune only anonymous volumes.
    /// Podman and older Docker prune every unused volume with no way to
    /// narrow it, so for those the anonymous unused volumes are removed
    /// one by one.
    pub async fn prune_volumes(
        &self,
        include_named: bool,
    ) -> Result<(), EngineError> {
        let can_narrow = prune_can_skip_named_volumes(self.info());

        if !include_named && !can_narrow {
            return self.remove_unused_anonymous_volumes().await;
        }

        let options =
            (include_named && can_narrow).then(|| PruneVolumesOptions {
                filters: Some(filter("all", "true")),
            });
        self.client.prune_volumes(options).await?;

        Ok(())
    }

    /// Remove every anonymous volume no container mounts, one by one.
    async fn remove_unused_anonymous_volumes(&self) -> Result<(), EngineError> {
        let containers = self.containers().await?;
        let volumes = self.volumes(&containers).await?;

        for volume in &volumes {
            if volume.is_anonymous() && volume.containers().is_empty() {
                self.remove_volume(volume.name()).await?;
            }
        }

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

/// Whether the engine's volume prune keeps named volumes unless `all=true`
/// is set: Docker 23 and later (API 1.42). Podman and older Docker remove
/// every unused volume, and older Docker rejects the `all` filter. An
/// unknown API version counts as old, which is the safe side.
fn prune_can_skip_named_volumes(info: &EngineInfo) -> bool {
    if info.kind() != EngineKind::Docker {
        return false;
    }
    let Some(version) = info.api_version().and_then(parse_api_version) else {
        return false;
    };

    (version.major_version, version.minor_version) >= PRUNE_ALL_VOLUMES_SINCE
}

/// One engine filter: `name` with one `value`.
fn filter(name: &str, value: &str) -> HashMap<String, Vec<String>> {
    HashMap::from([(name.to_string(), vec![value.to_string()])])
}

#[cfg(test)]
mod tests {
    use bollard::models::SystemVersion;

    use super::*;
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

    /// The info of an engine that answers `version`, reached at the same
    /// address as `fake`.
    fn info_at(fake: &EngineInfo, version: &str) -> EngineInfo {
        let answer: SystemVersion =
            serde_json::from_str(version).expect("version answer");

        EngineInfo::from_version(&answer, fake.address().clone())
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
    async fn prune_volumes_on_new_docker_sends_no_all_filter_by_default() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"VolumesDeleted":[],"SpaceReclaimed":0}"#,
        ));

        engine(&fake).prune_volumes(false).await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/volumes/prune"), "{path}");
        assert!(!requests[0].line.contains("all"), "{}", requests[0].line);
    }

    #[tokio::test]
    async fn prune_volumes_with_named_ones_sends_all_true_on_new_docker() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"VolumesDeleted":[],"SpaceReclaimed":0}"#,
        ));

        engine(&fake).prune_volumes(true).await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/volumes/prune"), "{path}");
        assert!(requests[0].line.contains("all"), "{}", requests[0].line);
        assert!(requests[0].line.contains("true"), "{}", requests[0].line);
    }

    /// The fake engine's answers for a prune that works by hand: the
    /// containers, the volumes, then one removal per volume in `removals`.
    fn by_hand_answers(removals: usize) -> Vec<(&'static str, String)> {
        let used = "a".repeat(64);
        let free = "b".repeat(64);
        let containers = format!(
            r#"[{{"Id":"c1","Names":["/web"],"State":"running","Mounts":[{{"Type":"volume","Name":"{used}"}}]}}]"#
        );
        let volumes = format!(
            r#"{{"Volumes":[
                {{"Name":"otty-fx-data","Driver":"local","Mountpoint":"/v/data","Labels":{{}},"Scope":"local","Options":{{}}}},
                {{"Name":"{used}","Driver":"local","Mountpoint":"/v/used","Labels":{{}},"Scope":"local","Options":{{}}}},
                {{"Name":"{free}","Driver":"local","Mountpoint":"/v/free","Labels":{{}},"Scope":"local","Options":{{}}}}
            ],"Warnings":[]}}"#
        );
        let mut answers = vec![("200 OK", containers), ("200 OK", volumes)];
        answers.extend((0..removals).map(|_| (NO_CONTENT, String::new())));

        answers
    }

    #[tokio::test]
    async fn podman_prune_without_named_ones_removes_only_unused_anonymous() {
        let fake = FakeEngine::start(by_hand_answers(1));
        let podman = info_at(
            fake.info(),
            r#"{"ApiVersion":"1.44","Components":[{"Name":"Podman Engine","Version":"5.8.8"}]}"#,
        );

        Engine::connect(&podman)
            .expect("engine")
            .prune_volumes(false)
            .await
            .expect("pruned");

        let requests = fake.requests();
        let lines: Vec<&str> = requests
            .iter()
            .map(|request| request.line.as_str())
            .collect();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines.iter().all(|line| !line.contains("/volumes/prune")));
        let (method, path) = method_and_path(lines[2]);
        assert_eq!(method, "DELETE");
        assert!(path.ends_with(&format!("/volumes/{}", "b".repeat(64))));
    }

    #[tokio::test]
    async fn docker_before_api_1_42_prunes_by_hand_without_named_ones() {
        let fake = FakeEngine::start(by_hand_answers(1));
        let old = info_at(
            fake.info(),
            r#"{"Version":"20.10.0","ApiVersion":"1.41"}"#,
        );

        Engine::connect(&old)
            .expect("engine")
            .prune_volumes(false)
            .await
            .expect("pruned");

        let requests = fake.requests();
        let lines: Vec<&str> = requests
            .iter()
            .map(|request| request.line.as_str())
            .collect();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines.iter().all(|line| !line.contains("/volumes/prune")));
    }

    #[tokio::test]
    async fn docker_with_unknown_api_version_also_prunes_by_hand() {
        let fake = FakeEngine::start(by_hand_answers(1));
        let unknown = info_at(fake.info(), r#"{"Version":"29.0.0"}"#);

        Engine::connect(&unknown)
            .expect("engine")
            .prune_volumes(false)
            .await
            .expect("pruned");

        let requests = fake.requests();
        assert_eq!(requests.len(), 3);
        assert!(
            requests
                .iter()
                .all(|request| !request.line.contains("/volumes/prune"))
        );
    }

    #[tokio::test]
    async fn podman_prune_with_named_ones_calls_prune_without_filters() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"VolumesDeleted":[],"SpaceReclaimed":0}"#,
        ));
        let podman = info_at(
            fake.info(),
            r#"{"ApiVersion":"1.44","Components":[{"Name":"Podman Engine","Version":"5.8.8"}]}"#,
        );

        Engine::connect(&podman)
            .expect("engine")
            .prune_volumes(true)
            .await
            .expect("pruned");

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/volumes/prune"), "{path}");
        assert!(!requests[0].line.contains("all"), "{}", requests[0].line);
    }

    #[tokio::test]
    async fn old_docker_prune_with_named_ones_calls_prune_without_filters() {
        let fake = FakeEngine::start(answer(
            "200 OK",
            r#"{"VolumesDeleted":[],"SpaceReclaimed":0}"#,
        ));
        let old = info_at(
            fake.info(),
            r#"{"Version":"20.10.0","ApiVersion":"1.41"}"#,
        );

        Engine::connect(&old)
            .expect("engine")
            .prune_volumes(true)
            .await
            .expect("pruned");

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert!(!requests[0].line.contains("all"), "{}", requests[0].line);
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
