//! The agent contract: the bindings of each version the host keeps, the
//! types of the newest version, and the calls into an agent part.
//!
//! A new version goes into its own module next to the old ones, such as
//! `v0_2`; the types below then come from it, and the old version's arm
//! of [`AgentCalls`] converts its types to them with `From`.

pub mod v0_1;

use std::sync::Arc;

use wasmtime::Store;
use wasmtime::component::{HasSelf, InstancePre, Linker};

/// The newest agent contract's types, which the rest of OTTY uses.
pub use crate::agents::agent::v0_1::exports::otty::agents_agent::agent::{
    Answer, AnswerCost, Answers, Place, PlaceItem, PlaceKind, Profile,
    ProfilePlace, ProfileStatus, RecordCounts, Session, TokenCount,
};
use crate::agents::agent::v0_1::exports::otty::agents_agent::agent::{
    Guest, GuestIndices,
};
/// The values of the `agents` domain both its contracts use, in the
/// newest version.
pub use crate::agents::agent::v0_1::otty::agents_agent::types::ReadError;
use crate::agents::prices::PriceList;
/// Who served an answer, which prices it.
pub use crate::agents::prices::ProviderRef;
use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::AgentVersion;
/// The host's types the agent contract uses.
pub use crate::operations::v0_1::otty::host::types::{
    Command, HostError, KeyAsk, Need, NeedAction,
};

/// The agent contract of one loaded plugin, in the version its
/// component exports.
#[derive(Clone)]
pub(crate) enum AgentExports {
    V0_1(GuestIndices),
}

impl AgentExports {
    /// The exports of `version` of the agent contract in the component of
    /// `instance_pre`.
    pub(crate) fn new(
        version: AgentVersion,
        instance_pre: &InstancePre<CallState>,
    ) -> wasmtime::Result<Self> {
        match version {
            AgentVersion::V0_1 => {
                Ok(Self::V0_1(GuestIndices::new(instance_pre)?))
            },
        }
    }
}

/// The calls of the agent contract into one agent part, within one
/// [`CallBatch`]. Each call has the deadline of its kind; an `Err` is a
/// trap, a missed deadline or the memory cap, while what the part itself
/// answers, such as a [`ReadError`], is in the `Ok` value.
pub struct AgentCalls<'batch, 'plugin> {
    batch: &'batch mut CallBatch<'plugin>,
    exports: AgentExports,
}

impl<'batch, 'plugin> AgentCalls<'batch, 'plugin> {
    pub(crate) fn new(
        batch: &'batch mut CallBatch<'plugin>,
        exports: AgentExports,
    ) -> Self {
        Self { batch, exports }
    }

    /// The agent's default config folder, the place marked default.
    pub fn profiles(&mut self) -> Result<Vec<ProfilePlace>, CallError> {
        let agent = self.batch.part().id().to_string();

        self.call(CallKind::Quick, |guest, store| {
            guest.call_profiles(store, &agent)
        })
    }

    /// Whether `profile` can be used: found, not logged in, or its folder
    /// is gone.
    pub fn detect(
        &mut self,
        profile: &Profile,
    ) -> Result<ProfileStatus, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_detect(store, profile)
        })
    }

    /// The agent's version written in the profile's newest files.
    pub fn version(
        &mut self,
        profile: &Profile,
    ) -> Result<Option<String>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_version(store, profile)
        })
    }

    /// Where the profile's sessions are.
    pub fn session_places(
        &mut self,
        profile: &Profile,
    ) -> Result<Vec<Place>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_session_places(store, profile)
        })
    }

    /// What one file or database of a session place tells about its
    /// sessions.
    pub fn read_sessions(
        &mut self,
        profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Result<Vec<Session>, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            guest.call_read_sessions(store, profile, item)
        })
    }

    /// Where the profile's answers are.
    pub fn spend_places(
        &mut self,
        profile: &Profile,
    ) -> Result<Vec<Place>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_spend_places(store, profile)
        })
    }

    /// Every answer in one file or database of a spend place, each priced
    /// by the part: the cost the agent wrote, else its tokens at the price
    /// `prices` gives when the part asks `prices.price-of` (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 3).
    pub fn read_answers(
        &mut self,
        profile: &Profile,
        item: &PlaceItem,
        prices: &Arc<dyn PriceList>,
    ) -> Result<Result<Answers, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            store.data_mut().set_prices(Some(Arc::clone(prices)));
            let answers = guest.call_read_answers(&mut *store, profile, item);
            store.data_mut().set_prices(None);

            answers
        })
    }

    /// The command that starts a new session, or continues `session`.
    pub fn launch(
        &mut self,
        profile: &Profile,
        session: Option<&str>,
    ) -> Result<Command, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_launch(store, profile, session)
        })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract.
    fn call<T>(
        &mut self,
        kind: CallKind,
        call: impl FnOnce(&Guest, &mut Store<CallState>) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();

        self.batch.call(kind, |store, instance| match &exports {
            AgentExports::V0_1(indices) => {
                let guest = indices.load(&mut *store, instance)?;

                call(&guest, store)
            },
        })
    }
}

/// The domain's shared values, `otty:agents-agent/types`, have no
/// function to serve.
impl crate::agents::agent::v0_1::otty::agents_agent::types::Host for CallState {}

/// Adds the domain's shared values, `otty:agents-agent/types`, and OTTY's
/// price list, `otty:agents-agent/prices`, to `linker`: a component
/// imports the values with either contract of the domain, and the price
/// list with its agent parts.
pub(crate) fn add_types_to_linker(
    linker: &mut Linker<CallState>,
) -> wasmtime::Result<()> {
    crate::agents::agent::v0_1::otty::agents_agent::types::add_to_linker::<
        CallState,
        HasSelf<CallState>,
    >(linker, |state| state)?;

    crate::agents::prices::add_to_linker(linker)
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::agents::prices::{MockPriceList, ModelPrice, TokenPrices};
    use crate::engine::PluginEngine;
    use crate::testing::{
        TempDir, fixture, load_fixture, load_template, template_env,
    };

    const AGENT: &str = "example-agent";

    /// A dollar a million input tokens, and nothing for the rest.
    fn dollar_per_million_input() -> ModelPrice {
        ModelPrice {
            standard: TokenPrices {
                input: 1_000_000,
                cached: 0,
                cache_written: 0,
                output: 0,
            },
            long_prompt: None,
        }
    }

    fn default_profile() -> Profile {
        let folder = template_env().home().join(".example-agent");

        Profile {
            agent: AGENT.to_string(),
            folder: folder.display().to_string(),
            is_default: true,
        }
    }

    #[test]
    fn the_template_gives_its_version_from_its_fixtures() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let version = batch
            .agent()
            .expect("an agent part")
            .version(&default_profile());

        assert_eq!(version, Ok(Some("1.4.2".to_string())));
    }

    /// Spec 2026-11-23, Decision 4: the template gives only its default
    /// folder, even with another of its folders in the home folder.
    #[test]
    fn the_template_gives_only_its_default_folder() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let places = batch
            .agent()
            .expect("an agent part")
            .profiles()
            .expect("the profiles");

        let listed: Vec<(&str, bool)> = places
            .iter()
            .map(|place| (place.folder.as_str(), place.is_default))
            .collect();
        assert_eq!(listed, [(default_profile().folder.as_str(), true)]);
    }

    #[test]
    fn a_variable_the_part_lists_reaches_it() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let env = template_env().with_var("EXAMPLE_AGENT_HOME", "/data/agent");
        let plugin = load_template(&engine, &env);
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let places = batch
            .agent()
            .expect("an agent part")
            .profiles()
            .expect("the profiles");

        assert_eq!(places[0].folder, "/data/agent");
        assert!(places[0].is_default);
    }

    #[test]
    fn what_the_part_logs_reaches_the_plugin_log() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let work = Profile {
            folder: template_env()
                .home()
                .join(".example-agent-work")
                .display()
                .to_string(),
            is_default: false,
            ..default_profile()
        };

        let version = batch.agent().expect("an agent part").version(&work);

        assert_eq!(version, Ok(None));
        assert!(
            plugin
                .log()
                .text()
                .contains("[warn] state.json is not in the shape"),
            "{}",
            plugin.log().text()
        );
    }

    /// Issue 25: the part names where its sessions are, and reads one
    /// file into the sessions it holds.
    #[test]
    fn the_template_reads_its_sessions_through_the_host() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let profile = default_profile();
        let item = PlaceItem {
            file: crate::FileInfo {
                path: format!("{}/sessions/s1.jsonl", profile.folder),
                size: 0,
                changed_ms: 0,
            },
            kind: PlaceKind::Files,
        };

        let mut calls = batch.agent().expect("an agent part");
        let places = calls.session_places(&profile).expect("the call ran");
        let sessions = calls
            .read_sessions(&profile, &item)
            .expect("the call ran")
            .expect("the sessions");

        let globs: Vec<&str> =
            places.iter().map(|place| place.glob.as_str()).collect();
        assert_eq!(globs, [format!("{}/sessions/*.jsonl", profile.folder)]);
        let found: Vec<(&str, Option<&str>, Option<&str>)> = sessions
            .iter()
            .map(|session| {
                (
                    session.id.as_str(),
                    session.folder.as_deref(),
                    session.title.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            found,
            [("s1", Some("/home/dev/shop"), Some("Fix the cart total"))]
        );
    }

    /// Issue 20: the part writes its needs in the language OTTY shows at
    /// each call: a language picked after the load reaches the next call.
    #[test]
    fn a_part_answers_in_the_language_of_each_call() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let env = template_env();
        let plugin = load_template(&engine, &env);
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let nobody = Profile {
            folder: env
                .home()
                .join(".example-agent-nobody")
                .display()
                .to_string(),
            is_default: false,
            ..default_profile()
        };
        let title_now = |batch: &mut crate::CallBatch<'_>| {
            let status = batch.agent().expect("an agent part").detect(&nobody);
            let Ok(ProfileStatus::NotLoggedIn(need)) = status else {
                panic!("not logged in: {status:?}");
            };
            assert!(matches!(need.action, NeedAction::Run(_)), "{need:?}");
            need.title
        };

        let before = title_now(&mut batch);
        engine.set_language("ru");
        let after = title_now(&mut batch);

        assert_eq!(before, "Needs login");
        assert_eq!(after, "Нужен вход");
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 2: the agent
    /// part knows nothing of accounts, so its contract has no login
    /// function.
    #[test]
    fn the_agent_part_has_no_login_calls() {
        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::from_file(
            &engine,
            fixture("template"),
        )
        .expect("the template compiles");

        let exports = component.component_type();
        let agent = exports
            .exports(&engine)
            .find(|(name, _)| name.starts_with("otty:agents-agent/agent@"))
            .map(|(_, item)| item.ty);
        let Some(wasmtime::component::types::ComponentItem::ComponentInstance(
            agent,
        )) = agent
        else {
            panic!("the template exports the agent contract");
        };
        let calls: Vec<&str> =
            agent.exports(&engine).map(|(name, _)| name).collect();

        assert!(calls.contains(&"detect"), "{calls:?}");
        assert!(!calls.contains(&"logins"), "{calls:?}");
        assert!(!calls.contains(&"login-refused"), "{calls:?}");
    }

    /// Decision 9: a profile with a login is found, with no word on whose
    /// login it is; one with no login is not logged in, with the part's
    /// need.
    #[test]
    fn a_profile_with_a_login_is_found_and_one_with_none_needs_a_login() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let env = template_env();
        let plugin = load_template(&engine, &env);
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let nobody = Profile {
            folder: env
                .home()
                .join(".example-agent-nobody")
                .display()
                .to_string(),
            is_default: false,
            ..default_profile()
        };

        let mut calls = batch.agent().expect("an agent part");
        let logged_in = calls.detect(&default_profile());
        let no_login = calls.detect(&nobody);

        assert!(
            matches!(logged_in, Ok(ProfileStatus::Found)),
            "{logged_in:?}"
        );
        let Ok(ProfileStatus::NotLoggedIn(need)) = no_login else {
            panic!("not logged in: {no_login:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert!(matches!(need.action, NeedAction::Run(_)), "{need:?}");
    }

    /// Decision 3: the part asks OTTY's price list for the price of each
    /// answer's model, through the host, and prices the tokens itself.
    #[test]
    fn the_template_prices_its_answers_at_the_price_otty_gives() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let profile = default_profile();
        let item = PlaceItem {
            file: crate::FileInfo {
                path: format!("{}/sessions/s1.jsonl", profile.folder),
                size: 0,
                changed_ms: 0,
            },
            kind: PlaceKind::Files,
        };
        let mut prices = MockPriceList::new();
        prices
            .expect_price_of()
            .withf(|model, provider| {
                model == "example-large"
                    && matches!(
                        provider,
                        Some(ProviderRef::Id(id)) if id == "example-cloud"
                    )
            })
            .returning(|_, _| Some(dollar_per_million_input()));
        let prices: Arc<dyn PriceList> = Arc::new(prices);

        let answers = batch
            .agent()
            .expect("an agent part")
            .read_answers(&profile, &item, &prices)
            .expect("the call ran")
            .expect("the answers");

        let costs: Vec<Option<u64>> = answers
            .answers
            .iter()
            .map(|answer| match answer.cost {
                Some(AnswerCost::AtApiPrices(cost)) => Some(cost),
                _ => None,
            })
            .collect();
        assert_eq!(costs, [Some(1_200), Some(800)]);
    }

    /// Decision 3: a model the price list has no price for has no cost.
    #[test]
    fn an_answer_with_no_price_has_no_cost() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let profile = default_profile();
        let item = PlaceItem {
            file: crate::FileInfo {
                path: format!("{}/sessions/s1.jsonl", profile.folder),
                size: 0,
                changed_ms: 0,
            },
            kind: PlaceKind::Files,
        };
        let mut prices = MockPriceList::new();
        prices.expect_price_of().returning(|_, _| None);
        let prices: Arc<dyn PriceList> = Arc::new(prices);

        let answers = batch
            .agent()
            .expect("an agent part")
            .read_answers(&profile, &item, &prices)
            .expect("the call ran")
            .expect("the answers");

        assert!(answers.answers.iter().all(|answer| answer.cost.is_none()));
    }

    #[test]
    fn a_provider_part_has_no_agent_calls() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");

        let error = batch.agent().err().expect("not an agent part");

        assert_eq!(
            error,
            CallError::OtherKind {
                part: PartRef::provider("example-cloud"),
                asked: otty_plugin_manifest::PartKind::Agent,
            }
        );
    }

    #[test]
    fn a_plugin_with_only_the_agent_contract_has_no_provider_calls() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "minimal");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        assert!(batch.provider().is_err());
        assert!(batch.agent().is_ok());
    }
}
