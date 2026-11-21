//! A login a provider refused (spec 2026-11-19-agent-plugins, issue 20).
//! A provider part reads with a login an agent keeps, such as a ChatGPT or
//! a Claude login, but does not know which agent keeps it: it says only
//! that the login was refused or expired. OTTY knows the profile and the
//! agent part that found the login, so it asks that part what the user
//! must do, and shows its need. When the part says nothing or can not
//! answer, OTTY says itself that the provider refused the login of the
//! profile, with no button: it can not know how that agent logs in again.

use otty_plugin_host::PluginPart;
use otty_plugin_host::agents::agent::Login;

use crate::agent::Agent;
use crate::error::SourceError;
use crate::profile::Profile;

/// A login the provider part `provider` refused, or saw expired, that no
/// agent part said what to do about: OTTY's own need names the provider
/// and the profile that holds the login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedLogin {
    provider: PluginPart,
    profile: Option<Profile>,
}

impl RefusedLogin {
    /// The provider part that refused the login.
    pub fn provider(&self) -> &PluginPart {
        &self.provider
    }

    /// The profile whose login it is, when the login was found in one.
    pub fn profile(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    /// A login `provider` refused, found in no profile yet.
    ///
    /// ```
    /// use otty_agents::RefusedLogin;
    /// use otty_plugin_host::PluginPart;
    ///
    /// let part = PluginPart::new("example", "Example", "0.1.0", "", "cloud");
    /// let refused = RefusedLogin::new(part);
    ///
    /// assert_eq!(refused.provider().part(), "cloud");
    /// assert!(refused.profile().is_none());
    /// ```
    pub fn new(provider: PluginPart) -> Self {
        Self {
            provider,
            profile: None,
        }
    }

    /// The same refusal of the login of `profile`.
    pub fn of_profile(mut self, profile: &Profile) -> Self {
        self.profile = Some(profile.clone());
        self
    }
}

/// `error` of a read with `login`, which `agent` found in `profile`: a
/// login the provider refused becomes the need `agent` gives for it,
/// about that profile; when it gives none, the refusal names the profile.
/// Any other error stays as it is.
pub(crate) fn to_holding_agent(
    error: SourceError,
    agent: &dyn Agent,
    profile: &Profile,
    login: &Login,
) -> SourceError {
    let SourceError::LoginRefused(refused) = error else {
        return error;
    };

    match agent.login_refused(profile, login) {
        Some(need) => {
            let need = need.about_profile(profile.config_dir());
            SourceError::NeedsUser(Box::new(need))
        },
        None => {
            SourceError::LoginRefused(Box::new(refused.of_profile(profile)))
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::agent::{
        Identity, KeyPlace, LoginKind, ProviderRef as PartProvider,
    };

    use super::*;
    use crate::command::AgentCommand;
    use crate::error::Unavailable;
    use crate::fixtures::login_need;
    use crate::testing::{MockAgent, made_up_part, profile};

    fn login() -> Login {
        Login {
            kind: LoginKind::Subscription,
            for_whom: PartProvider::Id("openai".to_string()),
            place: KeyPlace::EnvVar("MADE_UP_TOKEN".to_string()),
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        }
    }

    fn refused() -> SourceError {
        SourceError::LoginRefused(Box::new(RefusedLogin::new(made_up_part(
            "openai",
        ))))
    }

    #[test]
    fn a_refused_login_becomes_the_need_of_the_part_that_holds_it() {
        let holder = profile("OpenCode");
        let mut agent = MockAgent::new();
        agent.expect_login_refused().times(1).returning(|_, _| {
            let run = AgentCommand::new("opencode")
                .with_arg("auth")
                .with_arg("login");
            Some(login_need("opencode", "opencode", run))
        });

        let error = to_holding_agent(refused(), &agent, &holder, &login());

        let SourceError::NeedsUser(need) = error else {
            panic!("the need of the agent part: {error:?}");
        };
        assert_eq!(need.part().part(), "opencode");
        assert_eq!(need.profile(), Some(holder.config_dir()));
    }

    #[test]
    fn with_no_need_of_the_agent_part_the_refusal_names_the_profile() {
        let holder = profile("Codex");
        let mut agent = MockAgent::new();
        agent.expect_login_refused().times(1).returning(|_, _| None);

        let error = to_holding_agent(refused(), &agent, &holder, &login());

        let SourceError::LoginRefused(refused) = error else {
            panic!("OTTY's own refusal: {error:?}");
        };
        assert_eq!(refused.profile(), Some(&holder));
        assert_eq!(refused.provider().part(), "openai");
    }

    #[test]
    fn another_error_is_not_asked_about() {
        let holder = profile("Codex");
        let mut agent = MockAgent::new();
        agent.expect_login_refused().never();
        let gone = SourceError::NotAvailable(Unavailable::KeyGone);

        let error = to_holding_agent(gone.clone(), &agent, &holder, &login());

        assert_eq!(error, gone);
    }
}
