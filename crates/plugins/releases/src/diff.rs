//! The permissions a new version asks for compared with the ones the user
//! granted, part by part (spec 2026-11-19-agent-plugins, "Permissions":
//! "When it is shown").

use otty_plugin_manifest::{Manifest, PartKind, PartRef, PermissionLine};

use crate::plugins_file::InstalledPart;

/// The permission changes of a new version, part by part. Parts with no
/// change are left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionDiff {
    parts: Vec<PartChange>,
}

impl PermissionDiff {
    /// The parts that changed.
    pub fn parts(&self) -> &[PartChange] {
        &self.parts
    }

    /// Whether the new version needs the user's OK: it adds a permission
    /// or a part. Removed permissions and parts need none.
    pub fn asks_for_more(&self) -> bool {
        self.parts.iter().any(|change| match change {
            PartChange::New { .. } => true,
            PartChange::Changed { added, .. } => !added.is_empty(),
            PartChange::Removed { .. } => false,
        })
    }

    /// The changes from the parts the user granted, `granted`, to the
    /// parts of `manifest`.
    pub(crate) fn between(
        granted: &[InstalledPart],
        manifest: &Manifest,
    ) -> Self {
        let mut parts = Vec::new();

        for part in manifest.parts() {
            let lines = granted_lines(manifest, &part);
            let Some(old) = granted.iter().find(|old| old.part() == part)
            else {
                parts.push(PartChange::New { part, lines });
                continue;
            };

            let added: Vec<String> = lines
                .iter()
                .filter(|line| !old.granted().contains(line))
                .cloned()
                .collect();
            let removed: Vec<String> = old
                .granted()
                .iter()
                .filter(|line| !lines.contains(line))
                .cloned()
                .collect();
            if !added.is_empty() || !removed.is_empty() {
                parts.push(PartChange::Changed {
                    part,
                    added,
                    removed,
                });
            }
        }

        let new_parts = manifest.parts();
        for old in granted {
            if !new_parts.contains(&old.part()) {
                parts.push(PartChange::Removed { part: old.part() });
            }
        }

        Self { parts }
    }
}

/// What changed in one part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartChange {
    /// A part the installed version does not have, with all its
    /// permissions.
    New {
        /// The part.
        part: PartRef,
        /// Its permissions, in plain words.
        lines: Vec<String>,
    },
    /// A part the new version no longer has.
    Removed {
        /// The part.
        part: PartRef,
    },
    /// A part whose permissions changed.
    Changed {
        /// The part.
        part: PartRef,
        /// The permissions it asks for now ("new").
        added: Vec<String>,
        /// The permissions it no longer asks for ("no longer").
        removed: Vec<String>,
    },
}

/// The permissions of the part `part` of `manifest` in plain words, as the
/// user grants them.
pub(crate) fn granted_lines(
    manifest: &Manifest,
    part: &PartRef,
) -> Vec<String> {
    let as_text = |lines: Vec<PermissionLine>| -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    };

    match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| as_text(agent.permission_lines()))
            .unwrap_or_default(),
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| as_text(provider.permission_lines()))
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{template_manifest, template_parts};

    #[test]
    fn the_same_permissions_make_no_diff() {
        let manifest =
            Manifest::parse(&template_manifest("0.1.0")).expect("valid");

        let diff =
            PermissionDiff::between(&template_parts(&manifest), &manifest);

        assert_eq!(diff, PermissionDiff::default());
        assert!(!diff.asks_for_more());
    }

    #[test]
    fn added_and_removed_permissions_are_listed() {
        let old = Manifest::parse(&template_manifest("0.1.0")).expect("valid");
        let new = Manifest::parse(&template_manifest("0.2.0").replace(
            r#"http = ["api.example.com"]"#,
            r#"http = ["v2.example.com"]"#,
        ))
        .expect("valid");

        let diff = PermissionDiff::between(&template_parts(&old), &new);

        assert_eq!(
            diff.parts(),
            [PartChange::Changed {
                part: PartRef::provider("example-cloud"),
                added: vec!["Sends requests to `v2.example.com`".to_string()],
                removed: vec![
                    "Sends requests to `api.example.com`".to_string()
                ],
            }]
        );
        assert!(diff.asks_for_more());
    }

    #[test]
    fn only_removed_permissions_ask_for_nothing() {
        let old = Manifest::parse(&template_manifest("0.1.0")).expect("valid");
        let new = Manifest::parse(&template_manifest("0.2.0").replace(
            "env = [\"EXAMPLE_AGENT_HOME\"]",
            "env = [\"EXAMPLE_AGENT_HOME\"]\nwrite = []",
        ))
        .expect("valid");
        let mut granted = template_parts(&old);
        granted[0]
            .granted
            .push("Reads files in `~/.old/**`".to_string());

        let diff = PermissionDiff::between(&granted, &new);

        assert_eq!(
            diff.parts(),
            [PartChange::Changed {
                part: PartRef::agent("example-agent"),
                added: Vec::new(),
                removed: vec!["Reads files in `~/.old/**`".to_string()],
            }]
        );
        assert!(!diff.asks_for_more());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: a
    /// provider part gets no key found for its hosts, so a new host asks
    /// for nothing more.
    #[test]
    fn a_new_host_of_a_provider_asks_for_nothing_more() {
        let old = Manifest::parse(&template_manifest("0.1.0")).expect("valid");
        let new = Manifest::parse(&template_manifest("0.2.0").replace(
            r#"hosts = ["example.com"]"#,
            r#"hosts = ["example.com", "example.org"]"#,
        ))
        .expect("valid");

        let diff = PermissionDiff::between(&template_parts(&old), &new);

        assert!(!diff.asks_for_more());
    }

    #[test]
    fn new_and_removed_parts_are_listed() {
        let both = Manifest::parse(&template_manifest("0.1.0")).expect("valid");
        let agent_only_text = template_manifest("0.1.0");
        let agent_only = Manifest::parse(
            agent_only_text
                .split("[[agents.provider]]")
                .next()
                .expect("the agent half"),
        )
        .expect("valid");

        let grown =
            PermissionDiff::between(&template_parts(&agent_only), &both);
        let shrunk =
            PermissionDiff::between(&template_parts(&both), &agent_only);

        let [PartChange::New { part, lines }] = grown.parts() else {
            panic!("one new part: {grown:?}");
        };
        assert_eq!(*part, PartRef::provider("example-cloud"));
        assert!(
            lines.contains(&"Sends requests to `api.example.com`".to_string())
        );
        assert!(grown.asks_for_more());
        assert_eq!(
            shrunk.parts(),
            [PartChange::Removed {
                part: PartRef::provider("example-cloud"),
            }]
        );
        assert!(!shrunk.asks_for_more());
    }

    /// Issue 15 of spec 2026-11-19-agent-plugins: a plugin whose new
    /// version adds a provider part to an agent-only version, as the
    /// official Anthropic and OpenAI plugins did, shows the new part and
    /// waits for the user. The built example plugin stands for them.
    #[test]
    fn a_new_provider_part_of_a_built_plugin_asks_for_more() {
        let text =
            include_str!("../../host/tests/fixtures/example/otty-plugin.toml");
        let new = Manifest::parse(text).expect("valid");
        let agent_only = Manifest::parse(
            text.split("[[agents.provider]]")
                .next()
                .expect("the agent half"),
        )
        .expect("valid");

        let diff = PermissionDiff::between(&template_parts(&agent_only), &new);

        let [PartChange::New { part, .. }] = diff.parts() else {
            panic!("one new part: {diff:?}");
        };
        assert_eq!(*part, PartRef::provider("example-cloud"));
        assert!(diff.asks_for_more());
    }
}
