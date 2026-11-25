//! Which release to install: the newest one whose `.wasm` exports only
//! contract versions this OTTY knows (spec 2026-11-19-agent-plugins,
//! "Install and update flow", step 2, and "Updates"). The manifest names
//! no contract, so the `.wasm` of each release looked at is downloaded
//! and read.

use crate::error::InstallError;
use crate::github::{Release, RepositoryUrl, releases_of};
use crate::http::{HttpClient, HttpError};
use crate::package::{
    MANIFEST_FILE, MAX_MANIFEST_BYTES, MAX_WASM_BYTES, Package,
    check_contracts, download, parse_manifest, sha256_hex,
};
use crate::plugins_file::{InstalledPlugin, Source};

/// The largest releases list OTTY reads.
const MAX_RELEASES_LIST_BYTES: u64 = 8 * 1024 * 1024;

/// A newer release this OTTY can not run, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRelease {
    pub(crate) tag: String,
    pub(crate) reason: String,
}

impl SkippedRelease {
    /// The release tag, such as `v0.6.0`.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Why, such as "Needs a newer OTTY (agents.agent contract 0.2)".
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// The release picked, if any, and the newest release skipped because it
/// needs a newer OTTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picked {
    pub(crate) package: Option<Package>,
    pub(crate) too_new: Option<SkippedRelease>,
}

/// The semver releases of `repository`, newest first.
pub(crate) fn list_releases(
    http: &dyn HttpClient,
    repository: &RepositoryUrl,
) -> Result<Vec<Release>, InstallError> {
    let body = http
        .get(&repository.releases_api(), MAX_RELEASES_LIST_BYTES)
        .map_err(|error| match error {
            HttpError::RateLimited => InstallError::RateLimited,
            HttpError::NotFound => {
                InstallError::RepositoryNotFound(repository.to_string())
            },
            HttpError::Network(reason) => InstallError::Network(reason),
            _ => InstallError::UnexpectedAnswer(error.to_string()),
        })?;

    releases_of(&body)
}

/// The newest of `releases` (newest first) whose `.wasm` this OTTY can
/// run with its manifest. Pre-releases count only with
/// `include_prereleases`. With an `installed` plugin, only releases newer
/// than it count, and reaching its own release ends the search; a release
/// whose tag was installed before must still serve the same manifest.
pub(crate) fn pick_release(
    http: &dyn HttpClient,
    repository: &RepositoryUrl,
    releases: &[Release],
    installed: Option<&InstalledPlugin>,
    include_prereleases: bool,
) -> Result<Picked, InstallError> {
    let mut too_new = None;

    for release in releases {
        if release.is_prerelease() && !include_prereleases {
            continue;
        }
        if installed
            .is_some_and(|installed| release.version() < installed.version())
        {
            break;
        }
        let Some(asset) = release.asset(MANIFEST_FILE) else {
            continue;
        };

        let bytes = download(http, asset, MAX_MANIFEST_BYTES)?;

        if let Some(installed) = installed {
            check_unchanged(installed, release.tag(), &bytes)?;
            if release.version() == installed.version() {
                break;
            }
        }
        let manifest = parse_manifest(&bytes)?;
        if manifest.plugin().version() != release.version() {
            return Err(InstallError::VersionIsNotTag {
                tag: release.tag().to_string(),
                version: manifest.plugin().version().to_string(),
            });
        }

        let wasm_name = manifest.plugin().wasm();
        let Some(wasm_asset) = release.asset(wasm_name) else {
            return Err(InstallError::MissingFile(wasm_name.to_string()));
        };
        let wasm = download(http, wasm_asset, MAX_WASM_BYTES)?;

        match check_contracts(&manifest, &wasm) {
            Err(InstallError::NeedsNewerOtty(reason)) => {
                too_new.get_or_insert(SkippedRelease {
                    tag: release.tag().to_string(),
                    reason,
                });
                continue;
            },
            other => other?,
        }

        let source = Source::Release {
            url: repository.to_string(),
        };
        let package =
            Package::of_release(manifest, bytes, wasm, release, source);

        return Ok(Picked {
            package: Some(package),
            too_new,
        });
    }

    Ok(Picked {
        package: None,
        too_new,
    })
}

/// Refuses a manifest that differs from the one recorded for `tag`.
fn check_unchanged(
    installed: &InstalledPlugin,
    tag: &str,
    manifest: &[u8],
) -> Result<(), InstallError> {
    let recorded = installed
        .hashes()
        .get(tag)
        .and_then(|hashes| hashes.get(MANIFEST_FILE));

    match recorded {
        Some(hash) if *hash != sha256_hex(manifest) => {
            Err(InstallError::ReleaseChanged {
                tag: tag.to_string(),
                name: installed.name().to_string(),
            })
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        FakeGithub, FakeRelease, agent_only_wasm, needs_agent_0_2,
    };

    const REPO: &str = "someone/otty-plugin-example";

    fn repository() -> RepositoryUrl {
        RepositoryUrl::parse(&format!("https://github.com/{REPO}"))
            .expect("url")
    }

    fn pick(
        github: &FakeGithub,
        include_prereleases: bool,
    ) -> Result<Picked, InstallError> {
        let http = github.client();
        let releases = list_releases(&http, &repository())?;

        pick_release(&http, &repository(), &releases, None, include_prereleases)
    }

    fn picked_tag(picked: &Picked) -> Option<&str> {
        picked.package.as_ref().map(Package::tag)
    }

    #[test]
    fn the_newest_compatible_release_is_chosen() {
        let github = FakeGithub::new();
        github.publish(
            REPO,
            vec![
                FakeRelease::example("v0.1.0", "0.1.0"),
                FakeRelease::example("v0.3.0", "0.3.0"),
                FakeRelease::example("nightly", "0.9.0"),
                FakeRelease::example("v0.2.0", "0.2.0"),
            ],
        );

        let picked = pick(&github, false).expect("picked");

        assert_eq!(picked_tag(&picked), Some("v0.3.0"));
        assert_eq!(picked.too_new, None);
    }

    #[test]
    fn a_release_with_an_unknown_contract_version_is_skipped_with_the_reason() {
        let github = FakeGithub::new();
        github.publish(
            REPO,
            vec![
                FakeRelease::example("v0.1.0", "0.1.0"),
                needs_agent_0_2(FakeRelease::example("v0.2.0", "0.2.0")),
            ],
        );

        let picked = pick(&github, false).expect("picked");

        assert_eq!(picked_tag(&picked), Some("v0.1.0"));
        let skipped = picked.too_new.expect("skipped");
        assert_eq!(skipped.tag(), "v0.2.0");
        assert_eq!(
            skipped.reason(),
            "Needs a newer OTTY (agents.agent contract 0.2)"
        );
    }

    /// Issue 15 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// manifest names no contract, so the `.wasm` tells which ones the
    /// release serves, and a part it does not serve refuses the release.
    #[test]
    fn a_release_whose_wasm_does_not_serve_a_part_is_refused() {
        let github = FakeGithub::new();
        github.publish(
            REPO,
            vec![
                FakeRelease::example("v0.1.0", "0.1.0")
                    .with_file("example.wasm", &agent_only_wasm()),
            ],
        );

        let error = pick(&github, false).expect_err("refused");

        assert_eq!(
            error,
            InstallError::Compile(
                "The provider part `example-cloud` needs the \
                 `agents.provider` contract, but the .wasm does not export it"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_release_without_its_wasm_is_refused() {
        let github = FakeGithub::new();
        github.publish(
            REPO,
            vec![
                FakeRelease::example("v0.1.0", "0.1.0")
                    .without_file("example.wasm"),
            ],
        );

        let error = pick(&github, false).expect_err("refused");

        assert_eq!(
            error,
            InstallError::MissingFile("example.wasm".to_string())
        );
    }

    #[test]
    fn pre_releases_count_only_when_asked() {
        let github = FakeGithub::new();
        github.publish(
            REPO,
            vec![
                FakeRelease::example("v0.1.0", "0.1.0"),
                FakeRelease::example("v0.2.0-beta.1", "0.2.0-beta.1"),
                FakeRelease::example("v0.1.5", "0.1.5").marked_prerelease(),
            ],
        );

        let stable = pick(&github, false).expect("picked");
        let any = pick(&github, true).expect("picked");

        assert_eq!(picked_tag(&stable), Some("v0.1.0"));
        assert_eq!(picked_tag(&any), Some("v0.2.0-beta.1"));
    }

    #[test]
    fn a_manifest_version_that_is_not_the_tag_is_refused() {
        let github = FakeGithub::new();
        github.publish(REPO, vec![FakeRelease::example("v0.2.0", "0.1.0")]);

        let error = pick(&github, false).expect_err("refused");

        assert_eq!(
            error,
            InstallError::VersionIsNotTag {
                tag: "v0.2.0".to_string(),
                version: "0.1.0".to_string(),
            }
        );
    }

    #[test]
    fn github_failures_are_plain_errors() {
        let github = FakeGithub::new();
        let api = repository().releases_api();

        github.fail(&api, HttpError::RateLimited);
        let rate_limited = pick(&github, false).expect_err("refused");
        github.fail(&api, HttpError::NotFound);
        let missing = pick(&github, false).expect_err("refused");
        github.fail(&api, HttpError::Network("no route".to_string()));
        let offline = pick(&github, false).expect_err("refused");

        assert_eq!(rate_limited, InstallError::RateLimited);
        assert_eq!(
            missing.to_string(),
            format!(
                "The repository https://github.com/{REPO} is private or does not exist"
            )
        );
        assert_eq!(
            offline.to_string(),
            "OTTY could not reach GitHub: no route"
        );
    }
}
