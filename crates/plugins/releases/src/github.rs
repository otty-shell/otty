//! GitHub: the link of a plugin's repository, the one constant naming the
//! official organization, and the releases list of the REST API (spec
//! 2026-11-19-agent-plugins, Decisions 9 and 10).

use std::fmt;

use otty_plugin_manifest::PluginVersion;
use serde::Deserialize;

use crate::error::InstallError;

/// The GitHub organization of the OTTY team. A plugin whose repository is
/// under it is "Official". The organization may be renamed, so this is
/// the only place that names it.
pub const OFFICIAL_ORGANIZATION: &str = "otty-shell";

/// A GitHub repository, `https://github.com/<owner>/<repo>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryUrl {
    owner: String,
    repo: String,
}

impl RepositoryUrl {
    /// The user or organization that owns the repository.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository's name.
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// The repository of a link the user pasted, with or without `.git` or
    /// a trailing slash. Only GitHub links are accepted.
    ///
    /// ```
    /// use otty_plugin_releases::RepositoryUrl;
    ///
    /// let url = RepositoryUrl::parse(
    ///     "https://github.com/otty-shell/otty-plugin-opencode.git/",
    /// )
    /// .expect("a GitHub link");
    ///
    /// assert_eq!(url.owner(), "otty-shell");
    /// assert_eq!(url.repo(), "otty-plugin-opencode");
    /// assert!(url.is_official());
    /// assert!(RepositoryUrl::parse("https://gitlab.com/a/b").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, InstallError> {
        let bad_url = || InstallError::BadUrl(text.to_string());

        let path = text
            .trim()
            .strip_prefix("https://github.com/")
            .ok_or_else(bad_url)?;
        let path = path.strip_suffix('/').unwrap_or(path);
        let path = path.strip_suffix(".git").unwrap_or(path);

        let Some((owner, repo)) = path.split_once('/') else {
            return Err(bad_url());
        };
        if !is_github_name(owner) || !is_github_name(repo) {
            return Err(bad_url());
        }

        Ok(Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    /// Whether the repository is under [`OFFICIAL_ORGANIZATION`]. GitHub
    /// names ignore case.
    pub fn is_official(&self) -> bool {
        self.owner.eq_ignore_ascii_case(OFFICIAL_ORGANIZATION)
    }

    /// The REST API address of the repository's releases, newest first,
    /// as many as one page holds.
    pub(crate) fn releases_api(&self) -> String {
        let Self { owner, repo } = self;

        format!(
            "https://api.github.com/repos/{owner}/{repo}/releases?per_page=100"
        )
    }
}

impl fmt::Display for RepositoryUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { owner, repo } = self;

        write!(formatter, "https://github.com/{owner}/{repo}")
    }
}

/// One GitHub release whose tag is a semver version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Release {
    tag: String,
    version: PluginVersion,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

impl Release {
    /// The tag as GitHub has it, such as `v1.2.0`.
    pub(crate) fn tag(&self) -> &str {
        &self.tag
    }

    /// The tag as a version.
    pub(crate) fn version(&self) -> &PluginVersion {
        &self.version
    }

    /// Whether GitHub marks it a pre-release or its version has a
    /// pre-release part, such as `1.2.0-beta.1`.
    pub(crate) fn is_prerelease(&self) -> bool {
        self.prerelease || !self.version.semver().pre.is_empty()
    }

    /// The release's files.
    pub(crate) fn assets(&self) -> &[ReleaseAsset] {
        &self.assets
    }

    /// The file `name` of the release.
    pub(crate) fn asset(&self, name: &str) -> Option<&ReleaseAsset> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

/// One file of a release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ReleaseAsset {
    name: String,
    #[serde(rename = "browser_download_url")]
    url: String,
}

impl ReleaseAsset {
    /// The file name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Where the file is downloaded from.
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

/// The releases in a body of the REST API's releases list whose tag is
/// semver with an optional leading `v`, newest version first. Drafts and
/// other tags are left out.
pub(crate) fn releases_of(body: &[u8]) -> Result<Vec<Release>, InstallError> {
    let raw: Vec<RawRelease> =
        serde_json::from_slice(body).map_err(|error| {
            InstallError::UnexpectedAnswer(format!(
                "the releases list is not what GitHub sends: {error}"
            ))
        })?;

    let mut releases: Vec<Release> = raw
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = tag_version(&release.tag_name)?;

            Some(Release {
                tag: release.tag_name,
                version,
                prerelease: release.prerelease,
                assets: release.assets,
            })
        })
        .collect();
    releases.sort_by(|left, right| right.version.cmp(&left.version));

    Ok(releases)
}

/// The version of a release tag: semver, with one leading `v` allowed.
pub(crate) fn tag_version(tag: &str) -> Option<PluginVersion> {
    PluginVersion::parse(tag.strip_prefix('v').unwrap_or(tag))
}

/// One release as the REST API sends it.
#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

/// Whether `name` can be a GitHub owner or repository name.
fn is_github_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::releases_json;

    #[test]
    fn a_link_is_read_with_or_without_git_and_a_slash() {
        for text in [
            "https://github.com/someone/otty-plugin-x",
            "https://github.com/someone/otty-plugin-x/",
            "https://github.com/someone/otty-plugin-x.git",
            " https://github.com/someone/otty-plugin-x.git/ ",
        ] {
            let url = RepositoryUrl::parse(text).expect("a GitHub link");

            assert_eq!(url.owner(), "someone");
            assert_eq!(url.repo(), "otty-plugin-x");
            assert_eq!(
                url.to_string(),
                "https://github.com/someone/otty-plugin-x"
            );
        }
    }

    #[test]
    fn other_links_are_refused() {
        for text in [
            "https://gitlab.com/someone/plugin",
            "http://github.com/someone/plugin",
            "https://github.com/someone",
            "https://github.com/someone/plugin/releases",
            "https://github.com/../plugin",
            "github.com/someone/plugin",
        ] {
            let error = RepositoryUrl::parse(text).expect_err(text);

            assert_eq!(error, InstallError::BadUrl(text.to_string()));
        }
    }

    #[test]
    fn official_follows_the_one_constant() {
        let official = format!("https://github.com/{OFFICIAL_ORGANIZATION}/p");
        let upper = format!(
            "https://github.com/{}/p",
            OFFICIAL_ORGANIZATION.to_uppercase()
        );

        assert!(RepositoryUrl::parse(&official).expect("url").is_official());
        assert!(RepositoryUrl::parse(&upper).expect("url").is_official());
        assert!(
            !RepositoryUrl::parse("https://github.com/someone/p")
                .expect("url")
                .is_official()
        );
    }

    #[test]
    fn tags_that_are_not_semver_are_ignored_and_the_rest_ordered() {
        let body = releases_json(&[
            ("v1.2.0", false, &[]),
            ("nightly", false, &[]),
            ("1.10.0", false, &[]),
            ("v1.3", false, &[]),
            ("v2.0.0-beta.1", false, &[]),
        ]);

        let releases = releases_of(&body).expect("a list");

        let tags: Vec<&str> = releases.iter().map(Release::tag).collect();
        assert_eq!(tags, ["v2.0.0-beta.1", "1.10.0", "v1.2.0"]);
        assert!(releases[0].is_prerelease());
        assert!(!releases[1].is_prerelease());
    }

    #[test]
    fn a_tag_has_at_most_one_leading_v() {
        assert_eq!(
            tag_version("v1.2.0").map(|version| version.to_string()),
            Some("1.2.0".to_string())
        );
        assert_eq!(tag_version("vv1.2.0"), None);
        assert_eq!(tag_version("V1.2.0"), None);
    }

    #[test]
    fn a_body_that_is_not_a_list_is_an_unexpected_answer() {
        let error = releases_of(b"{\"message\": \"no\"}").expect_err("bad");

        assert!(matches!(error, InstallError::UnexpectedAnswer(_)));
    }
}
