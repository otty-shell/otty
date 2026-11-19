//! The admin keys older versions kept in `agents.json` by a profile, until
//! they are moved to its organization (spec Decision 7). Today an admin
//! key is kept with the organization it reads the bill of
//! (`saved_accounts.rs`).

use crate::admin_key::AdminKeyRef;
use crate::agent::AgentId;
use crate::profile::Profile;

/// An admin key an older version kept in the entry of a profile, not yet
/// moved to the organization of that profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OldAdminKey {
    pub(super) agent: AgentId,
    pub(super) owner: OldKeyOwner,
    pub(super) admin_key: AdminKeyRef,
}

impl OldAdminKey {
    /// Whether the key was kept in the entry of `profile`.
    pub(super) fn is_of(&self, profile: &Profile) -> bool {
        if profile.agent() != self.agent {
            return false;
        }

        match &self.owner {
            OldKeyOwner::DefaultProfile => profile.is_default(),
            OldKeyOwner::AddedProfile(name) => {
                !profile.is_default() && profile.name() == name
            },
        }
    }

    /// Whether the key was kept in the entry of the added profile of
    /// `agent` named `name`.
    pub(super) fn is_of_added(&self, agent: AgentId, name: &str) -> bool {
        self.agent == agent
            && self.owner == OldKeyOwner::AddedProfile(name.to_string())
    }
}

/// The profile entry an old admin key was kept in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OldKeyOwner {
    /// The changes to the default profile of the agent.
    DefaultProfile,
    /// The added profile with this name: names are unique.
    AddedProfile(String),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::account::Account;
    use crate::admin_key::AdminKeyRef;
    use crate::agent::AgentId;
    use crate::known_agents;
    use crate::profile::Profile;
    use crate::provider::Provider;
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{chatgpt_login, key_account, organization};

    const CLAUDE: AgentId = AgentId::new("claude-code");

    /// An older `agents.json` with an admin key in the default Claude
    /// Code profile and in an added one.
    const OLD_FILE: &str = r#"{
        "default_profiles": {
            "claude-code": {
                "admin_key": { "env_var": "ANTHROPIC_ADMIN_KEY" },
                "monthly_budget": 300.0
            }
        },
        "added_profiles": [
            {
                "agent": "claude-code",
                "name": "Claude Code · API",
                "config_dir": "/home/user/.claude-api",
                "admin_key": { "file": "/home/user/.config/admin-key" }
            }
        ]
    }"#;

    fn old_file() -> SavedProfiles {
        SavedProfiles::from_json(OLD_FILE, &known_agents())
            .expect("the old layout is read")
    }

    fn default_claude() -> Profile {
        Profile::default_for(
            CLAUDE,
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    fn claude_api() -> Profile {
        Profile::added(
            CLAUDE,
            "Claude Code · API",
            PathBuf::from("/home/user/.claude-api"),
        )
    }

    fn by_variable() -> AdminKeyRef {
        AdminKeyRef::EnvVar(String::from("ANTHROPIC_ADMIN_KEY"))
    }

    fn by_file() -> AdminKeyRef {
        AdminKeyRef::File(PathBuf::from("/home/user/.config/admin-key"))
    }

    /// The old file after its keys were moved, with `found` the accounts
    /// of the default profile and of the added one.
    fn moved(default: &[Account], added: &[Account]) -> SavedProfiles {
        let default_claude = default_claude();
        let claude_api = claude_api();

        old_file().with_admin_keys_moved(&[
            (&default_claude, default),
            (&claude_api, added),
        ])
    }

    #[test]
    fn the_admin_key_of_a_profile_moves_to_its_organization() {
        let home = organization("made-up-home-org");
        let work = organization("made-up-work-org");

        let saved = moved(
            std::slice::from_ref(&home),
            &[chatgpt_login("x"), work.clone()],
        );
        let text = saved.to_json().expect("the paths are text");

        assert_eq!(saved.admin_key_of(&home), Some(&by_variable()));
        assert_eq!(saved.admin_key_of(&work), Some(&by_file()));
        assert_eq!(
            SavedProfiles::from_json(&text, &known_agents())
                .expect("the text was just written"),
            saved
        );
        assert_eq!(
            text.matches("admin_key").count(),
            2,
            "only in the accounts: {text}"
        );
        assert!(!text.contains("budget"), "{text}");
    }

    #[test]
    fn an_organization_with_an_admin_key_keeps_it() {
        let organization = organization("made-up-org");

        let saved = moved(
            std::slice::from_ref(&organization),
            std::slice::from_ref(&organization),
        );
        let text = saved.to_json().expect("the paths are text");

        assert_eq!(saved.admin_key_of(&organization), Some(&by_variable()));
        assert_eq!(text.matches("admin_key").count(), 1, "{text}");
    }

    /// A key account is never saved: the old key stays in its entry until
    /// a later read finds the profile's organization.
    #[test]
    fn the_admin_key_of_a_profile_with_no_organization_stays_in_its_entry() {
        let key = key_account(Provider::Anthropic, "sk-ant-made-up-key");

        let saved = moved(std::slice::from_ref(&key), &[]);
        let text = saved.to_json().expect("the paths are text");

        assert_eq!(saved.admin_key_of(&key), None);
        assert_eq!(
            SavedProfiles::from_json(&text, &known_agents())
                .expect("the text was just written"),
            old_file()
        );
        assert!(!text.contains("accounts"), "{text}");
    }

    #[test]
    fn an_old_admin_key_follows_its_renamed_profile_and_goes_with_it() {
        let renamed = Profile::added(
            CLAUDE,
            "Claude Code · Console",
            PathBuf::from("/home/user/.claude-api"),
        );
        let organization = organization("made-up-org");

        let after_rename = old_file().with_added_replaced(
            CLAUDE,
            "Claude Code · API",
            renamed.clone(),
        );
        let moved = after_rename.clone().with_admin_keys_moved(&[(
            &renamed,
            std::slice::from_ref(&organization),
        )]);
        let removed =
            after_rename.without_added(CLAUDE, "Claude Code · Console");
        let removed_text = removed.to_json().expect("the paths are text");

        assert_eq!(moved.admin_key_of(&organization), Some(&by_file()));
        assert!(!removed_text.contains("admin-key"), "{removed_text}");
        assert!(removed_text.contains("ANTHROPIC_ADMIN_KEY"));
    }

    #[test]
    fn only_an_organization_takes_an_admin_key() {
        let key = key_account(Provider::Anthropic, "sk-ant-made-up-key");
        let login = chatgpt_login("made-up-account");

        let saved = SavedProfiles::default()
            .with_admin_key_of(&key, Some(by_file()))
            .with_admin_key_of(&login, Some(by_file()));

        assert_eq!(saved, SavedProfiles::default());
    }

    #[test]
    fn an_admin_key_is_replaced_or_removed() {
        let organization = organization("made-up-org");
        let saved = SavedProfiles::default()
            .with_admin_key_of(&organization, Some(by_file()));

        let replaced = saved
            .clone()
            .with_admin_key_of(&organization, Some(by_variable()));
        let removed = saved.with_admin_key_of(&organization, None);

        assert_eq!(replaced.admin_key_of(&organization), Some(&by_variable()));
        assert_eq!(removed, SavedProfiles::default());
    }

    #[test]
    fn an_account_of_an_unknown_provider_or_with_no_key_is_left_out() {
        let text = r#"{
            "accounts": [
                { "provider": "future", "organization_id": "a", "admin_key": { "file": "/k" } },
                { "provider": "anthropic", "organization_id": "b", "admin_key": { "keychain": "x" } },
                { "provider": "anthropic", "organization_id": "c" }
            ]
        }"#;

        let saved = SavedProfiles::from_json(text, &known_agents())
            .expect("the layout is read");

        assert_eq!(saved, SavedProfiles::default());
    }
}
