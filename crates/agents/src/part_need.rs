//! What a plugin part asks the user to do (spec 2026-11-19-agent-plugins,
//! issue 20): a need with its title, its text and the one thing its button
//! does, in the language the host told the part. OTTY shows it as it is,
//! with the name of the plugin that asks; it writes none of it itself.

use std::path::{Path, PathBuf};

use otty_plugin_host::PluginPart;
use otty_plugin_host::agents::agent::{
    Command as PartCommand, KeyAsk as PartKeyAsk, Need,
    NeedAction as PartAction,
};

use crate::command::AgentCommand;

/// One thing a plugin part asks the user to do, with one button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartNeed {
    part: PluginPart,
    profile: Option<PathBuf>,
    id: String,
    title: String,
    text: String,
    action: PartNeedAction,
}

impl PartNeed {
    /// The plugin part that asks.
    pub fn part(&self) -> &PluginPart {
        &self.part
    }

    /// The config folder of the profile an agent part gave the need
    /// about; `None` for a need of a provider part.
    pub fn profile(&self) -> Option<&Path> {
        self.profile.as_deref()
    }

    /// The part's id of the need: the same need from two reads has the
    /// same id, so it shows once.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What is needed, in a few words.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Why, and what the button does.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What the button does.
    pub fn action(&self) -> &PartNeedAction {
        &self.action
    }

    /// Whether `other` is the same need: of the same part of the same
    /// plugin, about the same profile, with the same id. The same need of
    /// two profiles is two needs: each button runs in its own folder.
    pub fn is_same_need(&self, other: &PartNeed) -> bool {
        self.part.plugin() == other.part.plugin()
            && self.part.part() == other.part.part()
            && self.profile == other.profile
            && self.id == other.id
    }

    /// The need `id` of `part`, with its texts and its action.
    ///
    /// ```
    /// use otty_agents::{PartNeed, PartNeedAction};
    /// use otty_plugin_host::PluginPart;
    ///
    /// let part = PluginPart::new("example", "Example", "0.1.0", "", "coder");
    /// let need = PartNeed::new(
    ///     part,
    ///     "new-key",
    ///     "The key was refused",
    ///     "Make a new key.",
    ///     PartNeedAction::OpenPage("https://example.com/keys".into()),
    /// );
    ///
    /// assert_eq!(need.part().name(), "Example");
    /// ```
    pub fn new(
        part: PluginPart,
        id: impl Into<String>,
        title: impl Into<String>,
        text: impl Into<String>,
        action: PartNeedAction,
    ) -> Self {
        Self {
            part,
            profile: None,
            id: id.into(),
            title: title.into(),
            text: text.into(),
            action,
        }
    }

    /// The same need, given about the profile in `config_dir`.
    pub fn about_profile(mut self, config_dir: &Path) -> Self {
        self.profile = Some(config_dir.to_path_buf());
        self
    }

    /// The need a part gave, `None` when OTTY can not show it: a page to
    /// open that is not an `https` page. The page of a key to ask for that
    /// is not one is left out.
    pub(crate) fn of_part(part: &PluginPart, need: &Need) -> Option<Self> {
        let action = match &need.action {
            PartAction::Run(command) => {
                PartNeedAction::Run(agent_command(command))
            },
            PartAction::AskKey(ask) => PartNeedAction::AskKey(KeyAsk::of(ask)),
            PartAction::OpenPage(page) if is_https(page) => {
                PartNeedAction::OpenPage(page.clone())
            },
            PartAction::CopyReport(report) => {
                PartNeedAction::CopyReport(report.clone())
            },
            _ => return None,
        };

        Some(Self::new(
            part.clone(),
            need.id.as_str(),
            need.title.as_str(),
            need.text.as_str(),
            action,
        ))
    }
}

/// What the button of a part's need does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartNeedAction {
    /// Run the command in a terminal tab, such as the agent's login.
    Run(AgentCommand),
    /// Show a field for a key the part asks for.
    AskKey(KeyAsk),
    /// Open this `https` page in the browser.
    OpenPage(String),
    /// Put this text on the clipboard.
    CopyReport(String),
}

/// A key a part asks the user for: OTTY keeps only where the user keeps
/// it and gives it back to the part under `slot`, or, with no slot, in
/// place of the account's own key (issue 26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyAsk {
    slot: Option<String>,
    label: String,
    hint: String,
    prefix: Option<String>,
    page: Option<String>,
}

impl KeyAsk {
    /// The part's own name for the key, such as `admin`; `None` for the
    /// account's own key, the key of an account the user added, which the
    /// new key replaces.
    pub fn slot(&self) -> Option<&str> {
        self.slot.as_deref()
    }

    /// The name of the field, such as "Admin key".
    pub fn label(&self) -> &str {
        &self.label
    }

    /// One line under the field.
    pub fn hint(&self) -> &str {
        &self.hint
    }

    /// What the key starts with, when the part tells.
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// The `https` page where the user makes the key.
    pub fn page(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// A key asked for under `slot`, with the field's `label` and `hint`.
    pub fn new(
        slot: impl Into<String>,
        label: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            slot: Some(slot.into()),
            label: label.into(),
            hint: hint.into(),
            prefix: None,
            page: None,
        }
    }

    /// A key asked for in place of the account's own key, with the
    /// field's `label` and `hint`.
    ///
    /// ```
    /// use otty_agents::KeyAsk;
    ///
    /// let ask = KeyAsk::for_account_key("API key", "Replaces the old key.");
    ///
    /// assert_eq!(ask.slot(), None);
    /// ```
    pub fn for_account_key(
        label: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            slot: None,
            label: label.into(),
            hint: hint.into(),
            prefix: None,
            page: None,
        }
    }

    /// The same ask with the key's `prefix` and the `page` of the key.
    pub fn with_prefix_and_page(
        mut self,
        prefix: Option<String>,
        page: Option<String>,
    ) -> Self {
        self.prefix = prefix;
        self.page = page.filter(|page| is_https(page));
        self
    }

    /// The ask a part gave.
    fn of(ask: &PartKeyAsk) -> Self {
        let ask_of_slot = match &ask.slot {
            Some(slot) => Self::new(slot, &ask.label, &ask.hint),
            None => Self::for_account_key(&ask.label, &ask.hint),
        };

        ask_of_slot.with_prefix_and_page(ask.prefix.clone(), ask.page.clone())
    }
}

/// A part's command as the app runs it.
pub(crate) fn agent_command(command: &PartCommand) -> AgentCommand {
    let mut agent_command = AgentCommand::new(command.program.as_str());
    for arg in &command.args {
        agent_command = agent_command.with_arg(arg.as_str());
    }
    for (name, value) in &command.env {
        agent_command = agent_command.with_env(name.as_str(), value.as_str());
    }

    agent_command
}

/// Whether `page` is an `https` address with a host.
fn is_https(page: &str) -> bool {
    page.strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part() -> PluginPart {
        PluginPart::new("example", "Example", "0.1.0", "", "example-agent")
    }

    fn need(action: PartAction) -> Need {
        Need {
            id: "not-logged-in".to_string(),
            title: "Needs login".to_string(),
            text: "Nobody is logged in.".to_string(),
            action,
        }
    }

    #[test]
    fn a_run_need_keeps_its_command_and_texts() {
        let command = PartCommand {
            program: "example-agent".to_string(),
            args: vec!["login".to_string()],
            env: vec![("EXAMPLE_HOME".to_string(), "/work".to_string())],
        };

        let shown = PartNeed::of_part(&part(), &need(PartAction::Run(command)))
            .expect("shown");

        assert_eq!(shown.title(), "Needs login");
        assert_eq!(shown.text(), "Nobody is logged in.");
        assert_eq!(
            shown.action(),
            &PartNeedAction::Run(
                AgentCommand::new("example-agent")
                    .with_arg("login")
                    .with_env("EXAMPLE_HOME", "/work")
            )
        );
    }

    /// Spec issue 20: `open-page` is `https` only.
    #[test]
    fn a_page_that_is_not_https_is_not_shown() {
        let plain = need(PartAction::OpenPage("http://example.com".into()));
        let file = need(PartAction::OpenPage("file:///etc/passwd".into()));
        let good = need(PartAction::OpenPage("https://example.com".into()));

        assert_eq!(PartNeed::of_part(&part(), &plain), None);
        assert_eq!(PartNeed::of_part(&part(), &file), None);
        assert!(PartNeed::of_part(&part(), &good).is_some());
    }

    #[test]
    fn the_page_of_a_key_to_ask_for_is_https_only() {
        let ask = |page: &str| {
            need(PartAction::AskKey(PartKeyAsk {
                slot: Some("admin".to_string()),
                label: "Admin key".to_string(),
                hint: "Reads the bill.".to_string(),
                prefix: Some("ex-admin-".to_string()),
                page: Some(page.to_string()),
            }))
        };

        let good = PartNeed::of_part(&part(), &ask("https://example.com/k"));
        let bad = PartNeed::of_part(&part(), &ask("javascript:alert(1)"));

        let Some(PartNeedAction::AskKey(good)) = good.map(|need| need.action)
        else {
            panic!("a key to ask for");
        };
        let Some(PartNeedAction::AskKey(bad)) = bad.map(|need| need.action)
        else {
            panic!("a key to ask for");
        };
        assert_eq!(good.page(), Some("https://example.com/k"));
        assert_eq!(good.prefix(), Some("ex-admin-"));
        assert_eq!(bad.page(), None);
    }

    /// Issue 26: a key asked for with no slot replaces the account's own
    /// key; one with a slot is kept under it.
    #[test]
    fn a_key_asked_for_with_no_slot_is_the_accounts_own_key() {
        let ask = |slot: Option<&str>| {
            need(PartAction::AskKey(PartKeyAsk {
                slot: slot.map(str::to_string),
                label: "API key".to_string(),
                hint: "Replaces the refused key.".to_string(),
                prefix: None,
                page: None,
            }))
        };

        let own = PartNeed::of_part(&part(), &ask(None)).expect("shown");
        let admin =
            PartNeed::of_part(&part(), &ask(Some("admin"))).expect("shown");

        let PartNeedAction::AskKey(own) = own.action() else {
            panic!("a key to ask for");
        };
        let PartNeedAction::AskKey(admin) = admin.action() else {
            panic!("a key to ask for");
        };
        assert_eq!(own.slot(), None);
        assert_eq!(
            own,
            &KeyAsk::for_account_key("API key", "Replaces the refused key.")
        );
        assert_eq!(admin.slot(), Some("admin"));
    }

    #[test]
    fn two_needs_are_the_same_by_plugin_and_id() {
        let page = || PartNeedAction::OpenPage("https://example.com".into());
        let one = PartNeed::new(part(), "new-key", "A", "a", page());
        let again = PartNeed::new(part(), "new-key", "B", "b", page());
        let other = PartNeed::new(part(), "bill-key", "A", "a", page());

        assert!(one.is_same_need(&again));
        assert!(!one.is_same_need(&other));
    }

    /// Issue 20: the same need of the same part about two profiles is two
    /// needs; about one profile it is one.
    #[test]
    fn the_same_need_of_two_profiles_is_two_needs() {
        let run = || PartNeedAction::Run(AgentCommand::new("example-agent"));
        let need = || PartNeed::new(part(), "login-refused", "A", "a", run());
        let home = need().about_profile(Path::new("/home/dev/.example"));
        let again = need().about_profile(Path::new("/home/dev/.example"));
        let work = need().about_profile(Path::new("/home/dev/.example-work"));
        let other_part = PartNeed::new(
            PluginPart::new("example", "Example", "0.1.0", "", "other"),
            "login-refused",
            "A",
            "a",
            run(),
        )
        .about_profile(Path::new("/home/dev/.example"));

        assert!(home.is_same_need(&again));
        assert!(!home.is_same_need(&work));
        assert!(!home.is_same_need(&other_part));
    }
}
