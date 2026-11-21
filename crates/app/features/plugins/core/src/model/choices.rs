//! The choice the install and update dialogs ask for each new part whose
//! agent id, provider id or host another plugin already serves (spec
//! 2026-11-19-agent-plugins, Decision 4): "Example Cloud is already served by
//! Example (official). Use: [Example (official)] / [this plugin]".

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::{Manifest, PartRef};
use otty_plugin_releases::{Conflict, Served, Trust, Winner};

use crate::state::PluginsState;
use crate::types::part_name;

/// The choice for one new part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceViewModel {
    /// The new part.
    pub(crate) part: PartRef,
    /// "Example Cloud is already served by Example (official)."
    pub(crate) line: String,
    /// The plugin that serves it now, such as "Example (official)".
    pub(crate) current: String,
    /// Whether the user gave it to the new plugin.
    pub(crate) use_new: bool,
}

/// One choice per new part of `conflicts`, in their order; `use_new`
/// lists the parts the user gave to the new plugin.
pub(crate) fn choices(
    state: &PluginsState,
    manifest: &Manifest,
    conflicts: &[Conflict],
    use_new: &[PartRef],
) -> Vec<ChoiceViewModel> {
    let mut choices: Vec<ChoiceViewModel> = Vec::new();

    for conflict in conflicts {
        if choices.iter().any(|choice| choice.part == *conflict.part()) {
            continue;
        }
        let current = winner_name(state, conflict.current());
        let name = part_name(Some(manifest), conflict.part());

        choices.push(ChoiceViewModel {
            line: i18n::filled(
                Key::TplPluginsAlreadyServed,
                &[("thing", &name), ("current", &current)],
            ),
            current,
            use_new: use_new.contains(conflict.part()),
            part: conflict.part().clone(),
        });
    }

    choices
}

/// The ids and hosts of `conflicts` whose parts the user gave to the new
/// plugin, as the installer takes them.
pub(crate) fn given_to_new(
    conflicts: &[Conflict],
    use_new: &[PartRef],
) -> Vec<Served> {
    conflicts
        .iter()
        .filter(|conflict| use_new.contains(conflict.part()))
        .map(|conflict| conflict.served().clone())
        .collect()
}

/// The name of who serves an id now: an installed plugin, "(official)"
/// when it is.
fn winner_name(state: &PluginsState, winner: &Winner) -> String {
    let Winner::Plugin(id) = winner;
    let Some(plugin) = state.plugin(id) else {
        return id.clone();
    };

    let installed = &plugin.installed;
    if installed.trust() == Trust::Official {
        i18n::filled(Key::TplPluginsOfficialName, &[("name", installed.name())])
    } else {
        installed.name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_releases::fixtures::{conflict, example_manifest};

    use super::*;
    use crate::testing::{agent, official_example, provider};

    fn conflicts() -> Vec<Conflict> {
        let example = Winner::Plugin(String::from("example"));
        vec![
            conflict(
                agent(),
                Served::Agent(String::from("example-agent")),
                example.clone(),
            ),
            conflict(
                provider(),
                Served::Provider(String::from("example-cloud")),
                example.clone(),
            ),
            conflict(
                provider(),
                Served::Host(String::from("example.com")),
                example,
            ),
        ]
    }

    /// One choice per new part, naming the official plugin that serves
    /// it now.
    #[test]
    fn each_new_part_another_plugin_serves_has_one_choice() {
        let state = PluginsState::new(vec![official_example()]);
        let manifest =
            Manifest::parse(&example_manifest("0.2.0")).expect("valid");

        let choices = choices(&state, &manifest, &conflicts(), &[agent()]);

        let current =
            i18n::filled(Key::TplPluginsOfficialName, &[("name", "Example")]);
        assert_eq!(
            choices,
            [
                ChoiceViewModel {
                    part: agent(),
                    line: i18n::filled(
                        Key::TplPluginsAlreadyServed,
                        &[("thing", "Example Agent"), ("current", &current)]
                    ),
                    current: current.clone(),
                    use_new: true,
                },
                ChoiceViewModel {
                    part: provider(),
                    line: i18n::filled(
                        Key::TplPluginsAlreadyServed,
                        &[("thing", "Example Cloud"), ("current", &current)]
                    ),
                    current,
                    use_new: false,
                },
            ]
        );
    }

    /// A part given to the new plugin takes every id and host of it.
    #[test]
    fn a_part_given_to_the_new_plugin_takes_its_ids_and_hosts() {
        let given = given_to_new(&conflicts(), &[provider()]);

        assert_eq!(
            given,
            [
                Served::Provider(String::from("example-cloud")),
                Served::Host(String::from("example.com")),
            ]
        );
    }
}
