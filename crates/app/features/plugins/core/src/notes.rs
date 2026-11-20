//! What Agents mode shows of the installed plugins besides the registry
//! (spec 2026-11-19-agent-plugins, "What the UI shows"): the parts that
//! are off, for "Read by: … (off)" and the need "<Part> of the <Plugin>
//! plugin is off [Turn on]", and the updates that wait for the user's OK.

use otty_app_domain::plugins::{OffPart, PartSide, PluginNotes, WaitingUpdate};
use otty_plugin_manifest::PartKind;

use crate::state::PluginsState;

/// The notes of the installed plugins of `state`.
pub(crate) fn notes(state: &PluginsState) -> PluginNotes {
    let mut off_parts = Vec::new();
    for plugin in state.plugins() {
        let installed = &plugin.installed;
        for part in installed.parts().iter().filter(|part| !part.is_on()) {
            let reference = part.part();
            off_parts.push(OffPart {
                plugin: installed.id().to_string(),
                plugin_name: installed.name().to_string(),
                version: installed.version().to_string(),
                side: match reference.kind() {
                    PartKind::Agent => PartSide::Agent,
                    PartKind::Provider => PartSide::Provider,
                },
                part: reference.id().to_string(),
                part_name: plugin.part_name(&reference),
                hosts: part.hosts().to_vec(),
            });
        }
    }

    let waiting_updates = state
        .plugins()
        .iter()
        .filter_map(|plugin| {
            let update = state.waiting_update(plugin.id())?;
            Some(WaitingUpdate {
                plugin: plugin.id().to_string(),
                name: plugin.installed.name().to_string(),
                from: update.from().to_string(),
                to: update.to().to_string(),
            })
        })
        .collect();

    PluginNotes {
        off_parts,
        waiting_updates,
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_releases::fixtures::{
        example_manifest, pending_update, with_part_on,
    };

    use super::*;
    use crate::state::PluginNews;
    use crate::testing::{provider, released_example};

    #[test]
    fn a_part_that_is_off_is_noted_with_its_plugin_and_hosts() {
        let mut plugin = released_example();
        plugin.installed = with_part_on(plugin.installed, &provider(), false);
        let state = PluginsState::new(vec![plugin]);

        let notes = notes(&state);

        assert_eq!(
            notes.off_parts,
            [OffPart {
                plugin: String::from("example"),
                plugin_name: String::from("Example"),
                version: String::from("0.1.0"),
                side: PartSide::Provider,
                part: String::from("example-cloud"),
                part_name: String::from("Example Cloud"),
                hosts: vec![String::from("example.com")],
            }]
        );
        assert!(notes.waiting_updates.is_empty());
    }

    #[test]
    fn an_update_that_waits_is_noted_with_both_versions() {
        let plugin = released_example();
        let newer = example_manifest("0.2.0").replace(
            "http = [\"api.example.com\"]",
            "http = [\"api.example.com\", \"x.example.com\"]",
        );
        let update = pending_update(&plugin.installed, &newer, Vec::new());
        let mut state = PluginsState::new(vec![plugin]);
        state.set_news("example", PluginNews::Waiting(Box::new(update)));

        let notes = notes(&state);

        assert_eq!(
            notes.waiting_updates,
            [WaitingUpdate {
                plugin: String::from("example"),
                name: String::from("Example"),
                from: String::from("0.1.0"),
                to: String::from("0.2.0"),
            }]
        );
    }
}
