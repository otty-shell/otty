//! Test values of the Plugins page: the template plugin installed from an
//! official or another release, or from a folder, and its two parts.

use std::path::PathBuf;

use otty_plugin_manifest::{Manifest, PartRef};
use otty_plugin_releases::fixtures::{example_manifest, installed};
use otty_plugin_releases::{OFFICIAL_ORGANIZATION, Source};

use crate::types::ListedPlugin;

/// The agent part of the template plugin.
pub(crate) fn agent() -> PartRef {
    PartRef::agent("example-agent")
}

/// The provider part of the template plugin.
pub(crate) fn provider() -> PartRef {
    PartRef::provider("example-cloud")
}

/// The template plugin at `version` as installed from `source`.
pub(crate) fn example_from(source: Source, version: &str) -> ListedPlugin {
    let text = example_manifest(version);

    ListedPlugin {
        installed: installed(&text, source),
        manifest: Some(Manifest::parse(&text).expect("valid")),
        folder: PathBuf::from(format!("/plugins/example/{version}")),
        icon: None,
    }
}

/// The template plugin installed from the releases of a repository of
/// the official organization.
pub(crate) fn official_example() -> ListedPlugin {
    example_from(
        Source::Release {
            url: format!(
                "https://github.com/{OFFICIAL_ORGANIZATION}/otty-plugin-example"
            ),
        },
        "0.1.0",
    )
}

/// The template plugin installed from a local folder.
pub(crate) fn local_example() -> ListedPlugin {
    example_from(
        Source::Local {
            folder: PathBuf::from("/work/otty-plugin-example"),
        },
        "0.1.0",
    )
}

/// The template plugin installed from its releases.
pub(crate) fn released_example() -> ListedPlugin {
    example_from(
        Source::Release {
            url: String::from("https://github.com/someone/otty-plugin-example"),
        },
        "0.1.0",
    )
}

/// Another plugin, `example-fork`, with the same parts as the template.
pub(crate) fn fork() -> ListedPlugin {
    let text = example_manifest("0.1.0")
        .replace("id = \"example\"", "id = \"example-fork\"")
        .replace("name = \"Example\"", "name = \"Example Fork\"");
    let source = Source::Release {
        url: String::from("https://github.com/other/otty-plugin-example-fork"),
    };

    ListedPlugin {
        installed: installed(&text, source),
        manifest: Some(Manifest::parse(&text).expect("valid")),
        folder: PathBuf::from("/plugins/example-fork/0.1.0"),
        icon: None,
    }
}
