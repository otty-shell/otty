//! OTTY's own logins in `agents.json` (spec
//! 2026-11-23-accounts-apart-from-profiles, "What OTTY stores"): each by
//! its id, the plugin, the provider part and the agent part. The folder
//! is not written: it is always `<data dir>/accounts/<id>/`, so an id
//! that is not a plain name is not read, or its folder would be another
//! place.

/// Whether `id` can name the folder of an own login: letters, digits,
/// `-` and `_` only, not empty.
pub(super) fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|char| {
            char.is_ascii_alphanumeric() || matches!(char, '-' | '_')
        })
}

#[cfg(test)]
mod tests {
    use crate::agent::AgentId;
    use crate::own_login::OwnLogin;
    use crate::provider::ProviderId;
    use crate::saved_profiles::SavedProfiles;

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    fn codex_login(id: &str) -> OwnLogin {
        OwnLogin::new(
            id,
            "openai",
            ProviderId::new("openai"),
            AgentId::new("codex"),
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, "What OTTY stores":
    /// an own login is kept by its id, plugin, provider part and agent
    /// part; its folder is not written.
    #[test]
    fn own_logins_are_saved_and_read_back() {
        let saved = SavedProfiles::default()
            .with_own_login(codex_login("6f0c1e2a"))
            .with_own_login(codex_login("7a1d2b3c"));

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert_eq!(
            loaded.own_logins(),
            [codex_login("6f0c1e2a"), codex_login("7a1d2b3c")]
        );
        assert!(!text.contains("accounts/"), "{text}");
    }

    #[test]
    fn the_text_lists_own_logins_as_the_spec_shows() {
        let text = r#"{
            "own_logins": [
                { "id": "6f0c1e2a", "plugin": "openai", "provider": "openai", "agent": "codex" }
            ]
        }"#;

        assert_eq!(load(text).own_logins(), [codex_login("6f0c1e2a")]);
    }

    /// The id names a folder under OTTY's data folder: an id that is not
    /// a plain name, such as one with `/` or `..`, would point elsewhere,
    /// so it is not read.
    #[test]
    fn an_own_login_whose_id_is_no_plain_name_is_not_read() {
        for id in ["", "..", "../x", "a/b", ".hidden", "a b"] {
            let text = format!(
                r#"{{"own_logins": [{{ "id": "{id}", "plugin": "openai", "provider": "openai", "agent": "codex" }}]}}"#
            );

            assert!(load(&text).own_logins().is_empty(), "{id:?}");
        }
    }

    #[test]
    fn removing_an_own_login_forgets_its_entry_only() {
        let saved = SavedProfiles::default()
            .with_own_login(codex_login("6f0c1e2a"))
            .with_own_login(codex_login("7a1d2b3c"));

        let removed = saved.without_own_login("6f0c1e2a");

        assert_eq!(removed.own_logins(), [codex_login("7a1d2b3c")]);
    }

    /// Spec "What OTTY stores": removing a plugin keeps its entries, so
    /// the own login of a plugin no entry of the registry names is kept.
    #[test]
    fn an_own_login_of_an_unknown_plugin_is_kept() {
        let text = r#"{
            "own_logins": [
                { "id": "a1", "plugin": "future", "provider": "future-cloud", "agent": "future-agent" }
            ]
        }"#;

        let loaded = load(text);

        assert_eq!(loaded.own_logins().len(), 1);
        assert_eq!(load(&loaded.to_json().expect("text")), loaded);
    }
}
