//! The permissions of a part as Plugins mode shows them (issue 35): each
//! one is a place, such as a folder, a command or a site, in the group of
//! what it reaches, such as Files or Sites. The page of a plugin and the
//! install and update dialogs show the same groups, one row per group in a
//! fixed order, a group with nothing left out.

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::{Manifest, PartKind, PartRef, PermissionLine};

/// What a permission reaches. A part shows one row per group, in this
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PermissionGroup {
    /// Files it reads.
    Files,
    /// Files it writes and deletes.
    ChangesFiles,
    /// SQLite databases it reads.
    Databases,
    /// Environment variables it reads.
    Variables,
    /// Programs it runs.
    Programs,
    /// Programs it runs in a hidden terminal.
    HiddenTerminal,
    /// Entries of the system keyring.
    Keyring,
    /// Files, databases and variables it reads logins or keys from.
    Logins,
    /// Sites it sends requests to.
    Sites,
    /// Provider logins and keys other plugins found in the agents.
    FromAgents,
    /// A line `plugins.json` keeps that the manifest no longer has, as it
    /// was kept.
    Other,
}

impl PermissionGroup {
    /// The short label of the group, such as "Files".
    pub(crate) fn label(self) -> &'static str {
        let key = match self {
            Self::Files => Key::PluginsGroupFiles,
            Self::ChangesFiles => Key::PluginsGroupChangesFiles,
            Self::Databases => Key::PluginsGroupDatabases,
            Self::Variables => Key::PluginsGroupVariables,
            Self::Programs => Key::PluginsGroupPrograms,
            Self::HiddenTerminal => Key::PluginsGroupHiddenTerminal,
            Self::Keyring => Key::PluginsGroupKeyring,
            Self::Logins => Key::PluginsGroupLogins,
            Self::Sites => Key::PluginsGroupSites,
            Self::FromAgents => Key::PluginsGroupFromAgents,
            Self::Other => Key::PluginsGroupOther,
        };

        i18n::t(key)
    }
}

/// How the update dialog marks a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaceMark {
    /// The new version asks for it.
    New,
    /// The new version no longer asks for it.
    NoLonger,
}

/// One place a permission reaches, in its group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlaceViewModel {
    /// The group it belongs to.
    pub(crate) group: PermissionGroup,
    /// The place: a path, a command, a site or a variable name, shown in
    /// the monospace font and without backticks; or words.
    pub(crate) text: String,
    /// Whether `text` is words, such as "Example Cloud accounts", shown in
    /// the UI font.
    pub(crate) is_words: bool,
    /// Muted words after the place, such as "once you allow the keyring".
    pub(crate) note: Option<&'static str>,
    /// What the update dialog says of it.
    pub(crate) mark: Option<PlaceMark>,
}

impl PlaceViewModel {
    /// The same place, marked `mark`.
    pub(crate) fn marked(mut self, mark: PlaceMark) -> Self {
        self.mark = Some(mark);
        self
    }

    /// `text` in `group`, in the monospace font.
    fn name(group: PermissionGroup, text: &str) -> Self {
        Self {
            group,
            text: text.to_string(),
            is_words: false,
            note: None,
            mark: None,
        }
    }
}

/// One row of a part: the label of a group and its places, one per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionGroupViewModel {
    /// "Files", "Sites"…
    pub(crate) label: &'static str,
    /// Its places, in the order of the manifest.
    pub(crate) places: Vec<PlaceViewModel>,
}

/// `places` in their groups: the groups in their order, the places of a
/// group in the order given, each place once.
pub(crate) fn grouped(
    mut places: Vec<PlaceViewModel>,
) -> Vec<PermissionGroupViewModel> {
    places.sort_by_key(|place| place.group);

    let mut groups: Vec<(PermissionGroup, Vec<PlaceViewModel>)> = Vec::new();
    for place in places {
        match groups.last_mut() {
            Some((group, kept)) if *group == place.group => {
                if !kept.contains(&place) {
                    kept.push(place);
                }
            },
            _ => groups.push((place.group, vec![place])),
        }
    }

    groups
        .into_iter()
        .map(|(group, places)| PermissionGroupViewModel {
            label: group.label(),
            places,
        })
        .collect()
}

/// Each permission of the part `part` of `manifest`, in the order of the
/// manifest: its own lines and, for a provider part, each site whose keys
/// it gets.
pub(crate) fn part_places(
    manifest: &Manifest,
    part: &PartRef,
) -> Vec<PlaceViewModel> {
    kept_lines(manifest, part)
        .into_iter()
        .map(|(_, place)| place)
        .collect()
}

/// The permission `line` the user granted, as `plugins.json` keeps it in
/// English, as a place: the same line of `manifest` when it has it, else
/// in [`PermissionGroup::Other`] as it was kept, without backticks.
pub(crate) fn granted_place(
    line: &str,
    manifest: Option<&Manifest>,
) -> PlaceViewModel {
    let known = manifest.and_then(|manifest| {
        manifest.parts().iter().find_map(|part| {
            kept_lines(manifest, part)
                .into_iter()
                .find(|(english, _)| english == line)
                .map(|(_, place)| place)
        })
    });

    known.unwrap_or_else(|| {
        PlaceViewModel::name(PermissionGroup::Other, &line.replace('`', ""))
    })
}

/// `line` as a place in its group.
fn line_place(line: &PermissionLine) -> PlaceViewModel {
    match line {
        PermissionLine::ReadsFiles(glob) => {
            PlaceViewModel::name(PermissionGroup::Files, glob)
        },
        PermissionLine::WritesFiles(glob) => {
            PlaceViewModel::name(PermissionGroup::ChangesFiles, glob)
        },
        PermissionLine::QueriesDatabases(glob) => {
            PlaceViewModel::name(PermissionGroup::Databases, glob)
        },
        PermissionLine::ReadsVariable(name) => {
            PlaceViewModel::name(PermissionGroup::Variables, name)
        },
        PermissionLine::RunsProgram(command) => {
            PlaceViewModel::name(PermissionGroup::Programs, command)
        },
        PermissionLine::StartsHiddenTerminal(command) => {
            PlaceViewModel::name(PermissionGroup::HiddenTerminal, command)
        },
        PermissionLine::ReadsKeyring { service, .. } => PlaceViewModel {
            note: Some(i18n::t(Key::PluginsPlaceKeyringNote)),
            ..PlaceViewModel::name(PermissionGroup::Keyring, service)
        },
        PermissionLine::ReadsLoginsInFiles(glob) => {
            PlaceViewModel::name(PermissionGroup::Logins, glob)
        },
        PermissionLine::ReadsLoginsInDatabase { databases, .. } => {
            PlaceViewModel::name(PermissionGroup::Logins, databases)
        },
        PermissionLine::ReadsKeyInVariable(name) => {
            PlaceViewModel::name(PermissionGroup::Logins, name)
        },
        PermissionLine::SendsRequests(host) => {
            PlaceViewModel::name(PermissionGroup::Sites, host)
        },
        PermissionLine::GetsProviderKeys(provider) => PlaceViewModel {
            is_words: true,
            ..PlaceViewModel::name(
                PermissionGroup::FromAgents,
                &i18n::filled(
                    Key::TplPluginsPlaceAccounts,
                    &[("provider", provider)],
                ),
            )
        },
    }
}

/// Each permission of `part` of `manifest` as `plugins.json` keeps it, in
/// English, with its place. A provider part also gets the keys found for
/// each of its sites.
fn kept_lines(
    manifest: &Manifest,
    part: &PartRef,
) -> Vec<(String, PlaceViewModel)> {
    let as_kept =
        |lines: Vec<PermissionLine>| -> Vec<(String, PlaceViewModel)> {
            lines
                .iter()
                .map(|line| (line.to_string(), line_place(line)))
                .collect()
        };

    match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| as_kept(agent.permission_lines()))
            .unwrap_or_default(),
        PartKind::Provider => {
            let Some(provider) = manifest
                .providers()
                .iter()
                .find(|provider| provider.id() == part.id())
            else {
                return Vec::new();
            };

            let mut lines = as_kept(provider.permission_lines());
            lines.extend(provider.hosts().iter().map(|host| {
                (
                    format!("Gets the keys found for `{host}`"),
                    PlaceViewModel::name(PermissionGroup::FromAgents, host),
                )
            }));

            lines
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_releases::fixtures::example_manifest;

    use super::*;
    use crate::testing::{agent, provider};

    fn manifest() -> Manifest {
        Manifest::parse(&example_manifest("0.1.0")).expect("valid")
    }

    fn texts(group: &PermissionGroupViewModel) -> Vec<&str> {
        group
            .places
            .iter()
            .map(|place| place.text.as_str())
            .collect()
    }

    /// Issue 35: an agent part's permissions are grouped by what they
    /// reach, in the order of the groups, places without backticks.
    #[test]
    fn an_agent_part_is_grouped_by_what_it_reaches() {
        let groups = grouped(part_places(&manifest(), &agent()));

        let labels: Vec<&str> =
            groups.iter().map(|group| group.label).collect();
        assert_eq!(
            labels,
            [
                i18n::t(Key::PluginsGroupFiles),
                i18n::t(Key::PluginsGroupVariables),
                i18n::t(Key::PluginsGroupLogins),
            ]
        );
        assert_eq!(
            texts(&groups[0]),
            ["~/.example-agent*/**", "{EXAMPLE_AGENT_HOME}/**"]
        );
        assert_eq!(texts(&groups[1]), ["EXAMPLE_AGENT_HOME"]);
        assert_eq!(
            texts(&groups[2]),
            [
                "~/.example-agent*/auth.json",
                "{EXAMPLE_AGENT_HOME}/auth.json"
            ]
        );
        assert!(
            groups
                .iter()
                .flat_map(|group| &group.places)
                .all(|place| !place.text.contains('`') && !place.is_words)
        );
    }

    /// Issue 35: a provider part sends requests to its sites and gets the
    /// accounts and the keys of its sites from the agents.
    #[test]
    fn a_provider_part_shows_its_sites_and_what_it_gets_from_agents() {
        let groups = grouped(part_places(&manifest(), &provider()));

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].label, i18n::t(Key::PluginsGroupSites));
        assert_eq!(texts(&groups[0]), ["api.example.com"]);
        assert_eq!(groups[1].label, i18n::t(Key::PluginsGroupFromAgents));
        let accounts = i18n::filled(
            Key::TplPluginsPlaceAccounts,
            &[("provider", "Example Cloud")],
        );
        assert_eq!(texts(&groups[1]), [accounts.as_str(), "example.com"]);
        assert!(groups[1].places[0].is_words);
        assert!(!groups[1].places[1].is_words);
    }

    /// Owner, 2026-11-22: a login in a database shows only the database,
    /// a key in a variable only the variable; a keyring entry says when it
    /// is read.
    #[test]
    fn logins_show_only_where_they_are() {
        let database = line_place(&PermissionLine::ReadsLoginsInDatabase {
            databases: String::from("~/.local/share/opencode/opencode.db"),
            table: String::from("credential"),
            column: String::from("value"),
        });
        let variable = line_place(&PermissionLine::ReadsKeyInVariable(
            String::from("OPENAI_API_KEY"),
        ));
        let keyring = line_place(&PermissionLine::ReadsKeyring {
            service: String::from("Claude Code-credentials"),
            user: String::from("me"),
        });

        assert_eq!(
            database,
            PlaceViewModel::name(
                PermissionGroup::Logins,
                "~/.local/share/opencode/opencode.db"
            )
        );
        assert_eq!(
            variable,
            PlaceViewModel::name(PermissionGroup::Logins, "OPENAI_API_KEY")
        );
        assert_eq!(keyring.group, PermissionGroup::Keyring);
        assert_eq!(keyring.text, "Claude Code-credentials");
        assert_eq!(keyring.note, Some(i18n::t(Key::PluginsPlaceKeyringNote)));
    }

    /// The lines `plugins.json` keeps in English are placed by the
    /// manifest; one it no longer has is kept in Other, without
    /// backticks.
    #[test]
    fn a_granted_line_is_placed_by_the_manifest() {
        let manifest = manifest();

        let known = granted_place(
            "Sends requests to `api.example.com`",
            Some(&manifest),
        );
        let unknown =
            granted_place("Reads files in `~/.old/**`", Some(&manifest));
        let unread = granted_place("Runs `old`", None);

        assert_eq!(
            known,
            PlaceViewModel::name(PermissionGroup::Sites, "api.example.com")
        );
        assert_eq!(
            unknown,
            PlaceViewModel::name(
                PermissionGroup::Other,
                "Reads files in ~/.old/**"
            )
        );
        assert_eq!(unread.group, PermissionGroup::Other);
    }

    /// A place a part reaches twice, such as a database with two login
    /// columns, shows once.
    #[test]
    fn a_place_shows_once_in_its_group() {
        let place = || PlaceViewModel::name(PermissionGroup::Logins, "a.db");

        let groups = grouped(vec![
            place(),
            PlaceViewModel::name(PermissionGroup::Files, "~/x/**"),
            place(),
        ]);

        assert_eq!(groups.len(), 2);
        assert_eq!(texts(&groups[1]), ["a.db"]);
    }
}
