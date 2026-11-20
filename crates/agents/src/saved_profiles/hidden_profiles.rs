//! The profiles the user hid, in `agents.json` (spec 2026-11-19,
//! Decision 7): OTTY does not read them.

use std::path::PathBuf;

use crate::agent::AgentId;
use crate::profile::Profile;

/// A hidden profile, kept by its agent and its config folder: a profile
/// OTTY found has no entry of its own, and a new name keeps it hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HiddenProfile {
    pub(super) agent: AgentId,
    pub(super) config_dir: PathBuf,
}

impl HiddenProfile {
    /// The hidden entry of `profile`.
    pub(super) fn of(profile: &Profile) -> Self {
        Self {
            agent: profile.agent().clone(),
            config_dir: profile.config_dir().to_path_buf(),
        }
    }

    /// Whether this is the entry of `profile`.
    pub(super) fn is_of(&self, profile: &Profile) -> bool {
        *profile.agent() == self.agent
            && profile.config_dir() == self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::agent::AgentId;
    use crate::profile::Profile;
    use crate::saved_profiles::SavedProfiles;

    fn claude_id() -> AgentId {
        AgentId::new("claude-code")
    }

    fn codex_id() -> AgentId {
        AgentId::new("codex")
    }

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    fn default_claude() -> Profile {
        Profile::default_for(
            claude_id(),
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    fn found_codex() -> Profile {
        Profile::found(
            codex_id(),
            "Work",
            PathBuf::from("/home/user/.codex-work"),
        )
    }

    /// Spec 2026-11-19, Decision 7: a hidden profile is kept by its agent
    /// and its folder, so a profile OTTY found, which has no entry of its
    /// own, stays hidden, and so does a profile after a new name.
    #[test]
    fn hidden_profiles_are_saved_by_agent_and_folder_and_read_back() {
        let saved = SavedProfiles::default()
            .with_profile_hidden(&default_claude(), true)
            .with_profile_hidden(&found_codex(), true);
        let renamed = Profile::added(
            codex_id(),
            "Codex · Office",
            PathBuf::from("/home/user/.codex-work"),
        );
        let other_folder = Profile::default_for(
            codex_id(),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        );

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert!(loaded.is_profile_hidden(&default_claude()));
        assert!(loaded.is_profile_hidden(&renamed));
        assert!(!loaded.is_profile_hidden(&other_folder));
    }

    #[test]
    fn the_text_lists_each_hidden_profile_by_its_agent_and_folder() {
        let text = r#"{
            "hidden_profiles": [
                { "agent": "claude-code", "config_dir": "/home/user/.claude" },
                { "agent": "codex", "config_dir": "/home/user/.codex-work" }
            ]
        }"#;

        let expected = SavedProfiles::default()
            .with_profile_hidden(&default_claude(), true)
            .with_profile_hidden(&found_codex(), true);
        assert_eq!(load(text), expected);
    }

    #[test]
    fn show_again_forgets_the_hidden_profile() {
        let hidden = SavedProfiles::default()
            .with_profile_hidden(&found_codex(), true)
            .with_profile_hidden(&found_codex(), true);

        let shown = hidden.clone().with_profile_hidden(&found_codex(), false);
        let text = shown.to_json().expect("the paths are text");

        assert!(hidden.is_profile_hidden(&found_codex()));
        assert_eq!(shown, SavedProfiles::default());
        assert!(!text.contains("hidden"), "{text}");
    }

    /// Spec 2026-11-19-agent-plugins, "What the UI shows": a hidden
    /// profile of an agent with no entry is kept, and is hidden again when
    /// its entry comes back.
    #[test]
    fn a_hidden_profile_of_an_unknown_agent_is_kept() {
        let text = r#"{
            "hidden_profiles": [
                { "agent": "future-agent", "config_dir": "/home/user/.future" }
            ]
        }"#;
        let future = Profile::found(
            AgentId::new("future-agent"),
            "Future",
            PathBuf::from("/home/user/.future"),
        );

        let loaded = load(text);

        assert!(loaded.is_profile_hidden(&future));
        assert_eq!(
            load(&loaded.to_json().expect("the paths are text")),
            loaded
        );
    }
}
