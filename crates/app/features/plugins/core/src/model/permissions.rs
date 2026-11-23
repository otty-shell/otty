//! The permissions of a part as Plugins mode shows them (issues 35, 36):
//! each one is a place, such as a folder, a command or a site, in the
//! group of what it reaches, such as Files or Sites, and each group says
//! in one line what it means. The Can access dialog of a part and the
//! install and update dialogs show the same groups, one row per group in
//! a fixed order, a group with nothing left out.

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
    /// Files a provider part reads inside the profile folders of one agent
    /// part of its plugin, the logins as handles.
    ProfileLogins,
    /// Sites it sends requests to.
    Sites,
    /// A line `plugins.json` keeps that the manifest no longer has, as it
    /// was kept.
    Other,
}

impl PermissionGroup {
    /// The short label of the group, such as "Files"; `agent` names the
    /// agent part of the profiles of [`Self::ProfileLogins`].
    pub(crate) fn label(self, agent: &str) -> String {
        let key = match self {
            Self::Files => Key::PluginsGroupFiles,
            Self::ChangesFiles => Key::PluginsGroupChangesFiles,
            Self::Databases => Key::PluginsGroupDatabases,
            Self::Variables => Key::PluginsGroupVariables,
            Self::Programs => Key::PluginsGroupPrograms,
            Self::HiddenTerminal => Key::PluginsGroupHiddenTerminal,
            Self::Keyring => Key::PluginsGroupKeyring,
            Self::Logins => Key::PluginsGroupLogins,
            Self::ProfileLogins => {
                return i18n::filled(
                    Key::TplPluginsGroupProfileLogins,
                    &[("agent", agent)],
                );
            },
            Self::Sites => Key::PluginsGroupSites,
            Self::Other => Key::PluginsGroupOther,
        };

        i18n::t(key).to_string()
    }

    /// One muted line on what the group means, such as "Read only". A
    /// line OTTY can not sort has none.
    pub(crate) fn meaning(self) -> Option<String> {
        let key = match self {
            Self::Files => Key::PluginsGroupFilesMeaning,
            Self::ChangesFiles => Key::PluginsGroupChangesFilesMeaning,
            Self::Databases => Key::PluginsGroupDatabasesMeaning,
            Self::Variables => Key::PluginsGroupVariablesMeaning,
            Self::Programs => Key::PluginsGroupProgramsMeaning,
            Self::HiddenTerminal => Key::PluginsGroupHiddenTerminalMeaning,
            Self::Keyring => Key::PluginsGroupKeyringMeaning,
            Self::Logins => Key::PluginsGroupLoginsMeaning,
            Self::ProfileLogins => Key::PluginsGroupProfileLoginsMeaning,
            Self::Sites => Key::PluginsGroupSitesMeaning,
            Self::Other => return None,
        };

        Some(i18n::t(key).to_string())
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
    /// the monospace font and without backticks.
    pub(crate) text: String,
    /// Muted words after the place, such as "once you allow the keyring".
    pub(crate) note: Option<&'static str>,
    /// The name of the agent part whose profiles a place of
    /// [`PermissionGroup::ProfileLogins`] lies in, such as "Codex": the
    /// places of each agent are a group of their own.
    pub(crate) profiles_of: Option<String>,
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
            note: None,
            profiles_of: None,
            mark: None,
        }
    }

    /// `glob` inside each profile folder of the agent part `agent`, with
    /// the muted words `note`.
    fn in_profiles(agent: &str, glob: &str, note: Key) -> Self {
        Self {
            note: Some(i18n::t(note)),
            profiles_of: Some(agent.to_string()),
            ..Self::name(PermissionGroup::ProfileLogins, glob)
        }
    }
}

/// One row of a part: the label of a group with what it means, and its
/// places, one per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionGroupViewModel {
    /// "Files", "Sites", "Logins in Codex profiles"…
    pub(crate) label: String,
    /// The muted line under the label, such as "Read only".
    pub(crate) meaning: Option<String>,
    /// Its places, in the order of the manifest.
    pub(crate) places: Vec<PlaceViewModel>,
}

/// `places` of a part in their groups: the groups in their order, the
/// places inside the profiles of each agent part a group of their own,
/// the places of a group in the order given, each place once.
pub(crate) fn grouped(
    mut places: Vec<PlaceViewModel>,
) -> Vec<PermissionGroupViewModel> {
    places.sort_by(|one, other| {
        (one.group, &one.profiles_of).cmp(&(other.group, &other.profiles_of))
    });

    let mut groups: Vec<Vec<PlaceViewModel>> = Vec::new();
    for place in places {
        match groups.last_mut() {
            Some(kept) if is_same_group(&kept[0], &place) => {
                if !kept.contains(&place) {
                    kept.push(place);
                }
            },
            _ => groups.push(vec![place]),
        }
    }

    groups
        .into_iter()
        .map(|places| {
            let group = places[0].group;
            let agent = places[0].profiles_of.clone().unwrap_or_default();

            PermissionGroupViewModel {
                label: group.label(&agent),
                meaning: group.meaning(),
                places,
            }
        })
        .collect()
}

/// Each permission of the part `part` of `manifest`, in the order of the
/// manifest.
pub(crate) fn part_places(
    manifest: &Manifest,
    part: &PartRef,
) -> Vec<PlaceViewModel> {
    kept_lines(manifest, part)
        .into_iter()
        .map(|(_, place)| place)
        .collect()
}

/// The permissions `granted` to a part, as `plugins.json` keeps them, in
/// their groups, placed by `manifest`.
pub(crate) fn granted_groups(
    granted: &[String],
    manifest: Option<&Manifest>,
) -> Vec<PermissionGroupViewModel> {
    let places = granted
        .iter()
        .map(|line| granted_place(line, manifest))
        .collect();

    grouped(places)
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
        PermissionLine::ReadsLoginsInProfiles { agent, glob } => {
            PlaceViewModel::in_profiles(
                agent,
                glob,
                Key::PluginsPlaceGivenAsHandles,
            )
        },
        PermissionLine::ReadsInProfiles { agent, glob } => {
            PlaceViewModel::in_profiles(agent, glob, Key::PluginsPlaceReadOnly)
        },
        PermissionLine::WritesInProfiles { agent, glob } => {
            PlaceViewModel::in_profiles(agent, glob, Key::PluginsPlaceMayChange)
        },
        PermissionLine::SendsRequests(host) => {
            PlaceViewModel::name(PermissionGroup::Sites, host)
        },
    }
}

/// Whether `one` and `other` show in one group: the same group, and for
/// the places in profiles, the profiles of the same agent part.
fn is_same_group(one: &PlaceViewModel, other: &PlaceViewModel) -> bool {
    one.group == other.group && one.profiles_of == other.profiles_of
}

/// Each permission of `part` of `manifest` as `plugins.json` keeps it, in
/// English, with its place.
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
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| as_kept(provider.permission_lines()))
            .unwrap_or_default(),
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
            groups.iter().map(|group| group.label.as_str()).collect();
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
                .all(|place| !place.text.contains('`'))
        );
    }

    /// Issue 35, 36: a provider part sends requests to its sites. Spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 5: it gets no
    /// key another part found, so no group of found logins shows.
    #[test]
    fn a_provider_part_shows_its_sites_and_no_found_logins() {
        let groups = grouped(part_places(&manifest(), &provider()));

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[1].label, i18n::t(Key::PluginsGroupSites));
        assert_eq!(texts(&groups[1]), ["api.example.com"]);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: what a
    /// provider part reads in the profiles of an agent part of its plugin
    /// is a group of its own, "Logins in Example Agent profiles": the
    /// login files given as handles and the other files read only, each
    /// inside the folder of a profile.
    #[test]
    fn what_a_provider_part_reads_in_profiles_is_a_group_of_its_own() {
        let groups = grouped(part_places(&manifest(), &provider()));

        let profiles = &groups[0];
        let notes: Vec<Option<&str>> =
            profiles.places.iter().map(|place| place.note).collect();

        assert_eq!(
            profiles.label,
            i18n::filled(
                Key::TplPluginsGroupProfileLogins,
                &[("agent", "Example Agent")]
            )
        );
        assert_eq!(
            profiles.meaning.as_deref(),
            Some(i18n::t(Key::PluginsGroupProfileLoginsMeaning))
        );
        assert_eq!(texts(profiles), ["auth.json", "sessions/*.jsonl"]);
        assert_eq!(
            notes,
            [
                Some(i18n::t(Key::PluginsPlaceGivenAsHandles)),
                Some(i18n::t(Key::PluginsPlaceReadOnly)),
            ]
        );
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: what a
    /// provider part writes and deletes inside each profile folder shows
    /// in the group of those profiles, noted so.
    #[test]
    fn what_a_part_writes_in_profiles_shows_in_their_group() {
        let place = line_place(&PermissionLine::WritesInProfiles {
            agent: "Claude Code".to_string(),
            glob: "projects/*.jsonl".to_string(),
        });

        assert_eq!(place.group, PermissionGroup::ProfileLogins);
        assert_eq!(place.profiles_of.as_deref(), Some("Claude Code"));
        assert_eq!(place.note, Some(i18n::t(Key::PluginsPlaceMayChange)));
    }

    /// The profiles of two agent parts are two groups, each named by its
    /// agent.
    #[test]
    fn the_profiles_of_each_agent_are_a_group_of_their_own() {
        let in_profiles = |agent: &str, glob: &str| {
            line_place(&PermissionLine::ReadsLoginsInProfiles {
                agent: agent.to_string(),
                glob: glob.to_string(),
            })
        };

        let groups = grouped(vec![
            in_profiles("Codex", "auth.json"),
            in_profiles("Claude Code", ".credentials.json"),
            in_profiles("Codex", "config.json"),
        ]);

        let labels: Vec<&str> =
            groups.iter().map(|group| group.label.as_str()).collect();
        let codex = i18n::filled(
            Key::TplPluginsGroupProfileLogins,
            &[("agent", "Codex")],
        );
        let claude = i18n::filled(
            Key::TplPluginsGroupProfileLogins,
            &[("agent", "Claude Code")],
        );
        assert_eq!(labels, [claude.as_str(), codex.as_str()]);
        assert_eq!(texts(&groups[1]), ["auth.json", "config.json"]);
    }

    /// Issue 36: under each label one muted line says what the group
    /// means. A line kept in `plugins.json` that OTTY can not sort has
    /// none.
    #[test]
    fn each_group_says_what_it_means() {
        let groups = grouped(vec![
            PlaceViewModel::name(PermissionGroup::Files, "~/.x/**"),
            PlaceViewModel::name(PermissionGroup::Sites, "x.com"),
            PlaceViewModel::name(PermissionGroup::Other, "Runs old"),
        ]);

        assert_eq!(
            groups[0].meaning.as_deref(),
            Some(i18n::t(Key::PluginsGroupFilesMeaning))
        );
        assert_eq!(
            groups[1].meaning.as_deref(),
            Some(i18n::t(Key::PluginsGroupSitesMeaning))
        );
        assert_eq!(groups[2].meaning, None);
    }

    #[test]
    fn every_group_but_other_has_its_meaning() {
        let all = [
            PermissionGroup::Files,
            PermissionGroup::ChangesFiles,
            PermissionGroup::Databases,
            PermissionGroup::Variables,
            PermissionGroup::Programs,
            PermissionGroup::HiddenTerminal,
            PermissionGroup::Keyring,
            PermissionGroup::Logins,
            PermissionGroup::ProfileLogins,
            PermissionGroup::Sites,
        ];

        let meanings: Vec<Option<String>> =
            all.iter().map(|group| group.meaning()).collect();

        assert!(meanings.iter().all(Option::is_some));
        assert_eq!(PermissionGroup::Other.meaning(), None);
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
