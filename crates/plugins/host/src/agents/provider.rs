//! The provider contract: the bindings of each version the host keeps,
//! the types of the newest version, and the calls into a provider part.
//!
//! A new version goes into its own module next to the old ones, such as
//! `v0_2`; the types below then come from it, and the old version's arm
//! of [`ProviderCalls`] converts its types to them with `From`.

pub mod v0_1;

use std::time::Instant;

use otty_plugin_manifest::LoginFolder;
use wasmtime::Store;
use wasmtime::component::{InstancePre, Resource};

use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::ProviderVersion;
use crate::keys::{KeyHandle, KeySource};
use crate::agents::agent::ReadError;
/// The newest provider contract's types, which the rest of OTTY uses.
pub use crate::agents::provider::v0_1::exports::otty::agents_provider::provider::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, ExtraUsage,
    FoundProfile, KeyLimit, LoginFacts, Money,
};
use crate::agents::provider::v0_1::exports::otty::agents_provider::provider::{
    Guest, GuestIndices,
};
use crate::operations::v0_1::otty::host::keys::GivenKey;
use crate::pacing::PaceKey;

/// The provider contract of one loaded plugin, in the version its `[api]`
/// names.
#[derive(Clone)]
pub(crate) enum ProviderExports {
    V0_1(GuestIndices),
}

impl ProviderExports {
    /// The exports of `version` of the provider contract in the component
    /// of `instance_pre`.
    pub(crate) fn new(
        version: ProviderVersion,
        instance_pre: &InstancePre<CallState>,
    ) -> wasmtime::Result<Self> {
        match version {
            ProviderVersion::V0_1 => {
                Ok(Self::V0_1(GuestIndices::new(instance_pre)?))
            },
        }
    }
}

/// The calls of the provider contract into one provider part, within one
/// [`CallBatch`]. The key of each call is given by its source, a place or
/// a key OTTY read itself: the part gets a handle to it for that
/// call only, never the key. The keys the user gave for the account after
/// a need of the part asked for them go with their slot, each also as a
/// handle. Which keys a part may get is the caller's choice (only
/// accounts of the part's own provider id or hosts).
///
/// Each call also gives the part its login folders: the folders of the
/// profiles in `found-in` of the call's login of the part's own plugin.
/// Inside them, for that call only, the part reads what the `logins` of
/// its manifest declare for the agent part of the profile (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 5).
pub struct ProviderCalls<'batch, 'plugin> {
    batch: &'batch mut CallBatch<'plugin>,
    exports: ProviderExports,
}

impl<'batch, 'plugin> ProviderCalls<'batch, 'plugin> {
    pub(crate) fn new(
        batch: &'batch mut CallBatch<'plugin>,
        exports: ProviderExports,
    ) -> Self {
        Self { batch, exports }
    }

    /// Whose account the key of `key` is.
    pub fn account(
        &mut self,
        key: &KeySource,
        found: &LoginFacts,
    ) -> Result<Result<AccountFacts, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(found, key, &[], |guest, store, key, _| {
            guest.call_account(store, &provider, key, found)
        })
    }

    /// The numbers of `account` right now, read any way the part can:
    /// with the key of `key` and the keys of `given`, each with its slot,
    /// or by asking the agent in a profile of `found`. A reading that ran
    /// a program or a hidden terminal is slow: it runs at most once in
    /// the five minutes of the limits read per part and the profiles of
    /// `found`, and the answer of the last reading is given within that
    /// time.
    pub fn numbers(
        &mut self,
        found: &LoginFacts,
        account: &AccountFacts,
        key: &KeySource,
        given: &[(String, KeySource)],
    ) -> Result<Result<AccountNumbers, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();
        let plugin = self.batch.plugin();
        let pace = PaceKey::new(self.batch.part(), &found_folders(found));
        let started_at = Instant::now();
        if let Some(answer) = plugin.pacing().recent(&pace, started_at) {
            return Ok(answer);
        }

        let (answer, is_slow) = self.call_with_keys(
            found,
            key,
            given,
            |guest, store, key, given| {
                let answer = guest.call_numbers(
                    &mut *store,
                    &provider,
                    found,
                    account,
                    key,
                    &given,
                )?;

                Ok((answer, store.data().has_started_programs()))
            },
        )?;

        if is_slow {
            plugin.pacing().remember(pace, started_at, answer.clone());
        }
        Ok(answer)
    }

    /// The bill of `account` over `days`, read with the key of `key` and
    /// the keys of `given`, each with its slot; `found` tells where the
    /// account was found.
    pub fn read_bill(
        &mut self,
        found: &LoginFacts,
        account: &AccountFacts,
        key: &KeySource,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<Result<Bill, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(found, key, given, |guest, store, key, given| {
            guest.call_read_bill(
                store, &provider, found, account, key, &given, days,
            )
        })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract, a handle to the key of `source`, lent to the plugin for
    /// this call only, a handle to each key of `given` with its slot,
    /// given to the plugin for this call, and the login folders of
    /// `found`.
    fn call_with_keys<T>(
        &mut self,
        found: &LoginFacts,
        source: &KeySource,
        given: &[(String, KeySource)],
        call: impl FnOnce(
            &Guest,
            &mut Store<CallState>,
            Resource<KeyHandle>,
            Vec<GivenKey>,
        ) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();
        let handle = KeyHandle::of(source.clone());
        let plugin = self.batch.plugin().manifest().plugin().id();
        let folders = login_folders(plugin, found);

        self.batch.call_in_folders(
            CallKind::Numbers,
            &folders,
            |store, instance| match &exports {
                ProviderExports::V0_1(indices) => {
                    let guest = indices.load(&mut *store, instance)?;
                    let owned = store.data_mut().table().push(handle)?;
                    let lent = Resource::new_borrow(owned.rep());
                    let mut given_keys = Vec::new();
                    for (slot, source) in given {
                        let key = KeyHandle::of(source.clone());
                        given_keys.push(GivenKey {
                            slot: slot.clone(),
                            key: store.data_mut().table().push(key)?,
                        });
                    }

                    let result = call(&guest, store, lent, given_keys);
                    store.data_mut().table().delete(owned)?;

                    result
                },
            },
        )
    }
}

/// The login folders of a call about `found` into a part of the plugin
/// `plugin`: the folders of its profiles of that plugin. The part's
/// `logins` then tell what it reads in each, by the profile's agent part;
/// a profile of another plugin is no login folder.
fn login_folders(plugin: &str, found: &LoginFacts) -> Vec<LoginFolder> {
    found
        .found_in
        .iter()
        .filter(|profile| profile.plugin == plugin)
        .map(|profile| LoginFolder::new(&profile.agent, &profile.folder))
        .collect()
}

/// The folders of the profiles of `found`, one per line: what the pace
/// of a slow reading is kept by.
fn found_folders(found: &LoginFacts) -> String {
    let folders: Vec<&str> = found
        .found_in
        .iter()
        .map(|profile| profile.folder.as_str())
        .collect();

    folders.join("\n")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_plugin_manifest::{Environment, LoginFolder, PartRef};

    use super::*;
    use crate::agents::agent::Identity;
    use crate::calls::CallKind;
    use crate::https::MockHttpsSend;
    use crate::keys::KeyText;
    use crate::operations::v0_1::otty::host::files::Host as _;
    use crate::operations::v0_1::otty::host::http::Response;
    use crate::operations::v0_1::otty::host::keys::KeyPlace;
    use crate::operations::v0_1::otty::host::types::{HostError, NeedAction};
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{
        TempDir, debug_err, engine_with, found_profile, load_template,
        template_env, write_file,
    };

    const KEY: &str = "ex-made-up-key-0001";

    /// The template's provider part.
    const PROVIDER: &str = "example-cloud";

    /// A folder of a profile of the template's agent part that no glob of
    /// the template names, with its login in `auth.json` and its version
    /// in `state.json`.
    fn login_folder() -> TempDir {
        let folder = TempDir::new();
        write_file(
            &folder.path().join("auth.json"),
            format!(r#"{{"api_key": "{KEY}"}}"#).as_bytes(),
        );
        write_file(
            &folder.path().join("state.json"),
            br#"{"last_version": "1.4.2"}"#,
        );

        folder
    }

    /// What OTTY found of a login in the profiles of the plugins and
    /// folders of `profiles`, of the template's agent part.
    fn found_in(profiles: &[(&str, &Path)]) -> LoginFacts {
        let found_in = profiles
            .iter()
            .map(|(plugin, folder)| {
                let folder = folder.to_str().expect("a text path");

                found_profile(plugin, "example-agent", folder)
            })
            .collect();

        LoginFacts {
            found_in,
            ..key_facts()
        }
    }

    /// The numbers the template's provider part reads about a login of
    /// `found`, with a lent key that is not there: the part reads the key
    /// of a login folder itself. `https` serves its requests.
    // A `ReadError` may hold a need, a record the contract can not box.
    #[allow(clippy::result_large_err)]
    fn numbers_of(
        found: &LoginFacts,
        https: MockHttpsSend,
    ) -> Result<AccountNumbers, ReadError> {
        let cache = TempDir::new();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");
        let unset =
            KeySource::Place(KeyPlace::EnvVar("EXAMPLE_CLOUD_KEY".to_string()));

        batch
            .provider()
            .expect("a provider part")
            .numbers(found, &no_account(), &unset, &[])
            .expect("the call")
    }

    /// An account the provider part knows nothing of.
    fn no_account() -> AccountFacts {
        AccountFacts {
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        }
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: in a
    /// call about a login found in a profile of its own plugin, the part
    /// reads `auth.json` in that folder itself, its key as a handle, also
    /// when no glob of the part names the folder.
    #[test]
    fn in_a_call_with_a_folder_the_part_reads_the_login_there() {
        let folder = login_folder();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.url == "https://api.example.com/v1/balance"
                    && sent.headers.iter().any(|(name, value)| {
                        name == "authorization"
                            && *value == format!("Bearer {KEY}")
                    })
            })
            .times(1)
            .returning(|_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"balance": 7.5, "currency": "USD"}"#.to_vec(),
                })
            });

        let numbers =
            numbers_of(&found_in(&[("example", folder.path())]), https);

        let Ok(numbers) = numbers else {
            panic!("the part's answer: {numbers:?}");
        };
        assert_eq!(numbers.left.map(|left| left.amount), Some(7.5));
    }

    /// A profile of another plugin gives the part no login folder: it can
    /// not read the login there, so it reads nothing and sends nothing.
    #[test]
    fn a_profile_of_another_plugin_is_no_login_folder() {
        let folder = login_folder();
        let mut https = MockHttpsSend::new();
        https.expect_send().never();

        let numbers =
            numbers_of(&found_in(&[("another", folder.path())]), https);

        let Ok(numbers) = numbers else {
            panic!("the part's answer: {numbers:?}");
        };
        assert!(numbers.left.is_none());
    }

    /// What the template's provider part gets when it reads `file` with
    /// its keys as handles, and the bytes of `file`, in a call with the
    /// login folders `folders`.
    fn reads_in_call(
        folders: &[LoginFolder],
        file: &Path,
    ) -> (Result<String, String>, Result<Vec<u8>, String>) {
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");
        let path = file.to_str().expect("a text path").to_string();

        batch
            .call_in_folders(CallKind::Quick, folders, |store, _| {
                let state = store.data_mut();
                let with_keys = state
                    .read_json_with_keys(path.clone())
                    .map(|document| document.json);
                let bytes = state.read(path, 1024);

                Ok((debug_err(with_keys), debug_err(bytes)))
            })
            .expect("the call")
    }

    fn not_allowed(path: &Path) -> String {
        let error = HostError::NotAllowed(path.display().to_string());

        format!("{error:?}")
    }

    /// The login file of a login folder is read only with its key as a
    /// handle; the same read in a call with no folder, or with a folder of
    /// another agent part, is refused.
    #[test]
    fn a_login_file_is_read_only_in_a_call_with_its_folder() {
        let folder = login_folder();
        let auth = folder.path().join("auth.json");
        let own = LoginFolder::new("example-agent", folder.path());
        let of_another_agent = LoginFolder::new("other-agent", folder.path());

        let in_folder = reads_in_call(&[own], &auth);
        let with_no_folder = reads_in_call(&[], &auth);
        let in_another = reads_in_call(&[of_another_agent], &auth);

        assert_eq!(in_folder.0, Ok(r#"{"api_key":{"$key":0}}"#.to_string()));
        assert_eq!(in_folder.1, Err(not_allowed(&auth)));
        assert_eq!(with_no_folder.0, Err(not_allowed(&auth)));
        assert_eq!(in_another.0, Err(not_allowed(&auth)));
    }

    /// A file of the login folder the part does not declare is refused.
    #[test]
    fn a_file_of_the_folder_not_declared_is_refused() {
        let folder = login_folder();
        let state = folder.path().join("state.json");
        let own = LoginFolder::new("example-agent", folder.path());

        let (with_keys, bytes) = reads_in_call(&[own], &state);

        assert_eq!(with_keys, Err(not_allowed(&state)));
        assert_eq!(bytes, Err(not_allowed(&state)));
    }

    /// What OTTY knows of a key it read itself: an API key, found in no
    /// profile.
    fn key_facts() -> LoginFacts {
        LoginFacts {
            kind: crate::agents::agent::LoginKind::ApiKey,
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
            found_in: Vec::new(),
        }
    }

    /// The money left the template's provider part reads with the key of
    /// `key`, the plugin's variables from `env`; the one request it sends
    /// must carry [`KEY`].
    fn money_left_with(key: &KeySource, env: &Environment) -> f64 {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.url == "https://api.example.com/v1/balance"
                    && sent.headers.iter().any(|(name, value)| {
                        name == "authorization"
                            && *value == format!("Bearer {KEY}")
                    })
            })
            .times(1)
            .returning(|_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"balance": 12.5, "currency": "USD"}"#.to_vec(),
                })
            });
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, env);
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");
        let account = AccountFacts {
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        };

        let numbers = batch.provider().expect("a provider part").numbers(
            &key_facts(),
            &account,
            key,
            &[],
        );

        let Ok(Ok(numbers)) = numbers else {
            panic!("the part's answer: {numbers:?}");
        };
        let left = numbers.left.expect("the money left");
        assert_eq!(left.currency, "USD");
        left.amount
    }

    #[test]
    fn the_template_provider_reads_the_balance_with_its_key_handle() {
        let env = template_env().with_var("EXAMPLE_CLOUD_KEY", KEY);
        let key =
            KeySource::Place(KeyPlace::EnvVar("EXAMPLE_CLOUD_KEY".to_string()));

        assert_eq!(money_left_with(&key, &env), 12.5);
    }

    /// Issue 20: a key the user gave for a slot a need asked for reaches
    /// the next `read-bill` as a handle with that slot, and the host fills
    /// it into the part's request.
    #[test]
    fn a_given_key_reaches_the_bill_with_its_slot() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.url.starts_with("https://api.example.com/v1/bill?")
                    && sent.headers.iter().any(|(name, value)| {
                        name == "authorization"
                            && value == "Bearer ex-admin-made-up-0001"
                    })
            })
            .times(1)
            .returning(|_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"days": [{"day": "2026-11-02", "usd": 4.0}]}"#
                        .to_vec(),
                })
            });
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");
        let account = AccountFacts {
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        };
        let key = KeySource::Given(KeyText::new(KEY).expect("a key"));
        let admin = KeyText::new("ex-admin-made-up-0001").expect("a key");
        let days = DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        };

        let bill = batch.provider().expect("a provider part").read_bill(
            &key_facts(),
            &account,
            &key,
            &[("admin".to_string(), KeySource::Given(admin))],
            &days,
        );

        let Ok(Ok(bill)) = bill else {
            panic!("the bill: {bill:?}");
        };
        assert_eq!(bill.days[0].cost.amount, 4.0);
    }

    /// With no key under its slot, the part asks the user for one.
    #[test]
    fn a_bill_with_no_given_key_is_a_need_of_the_part() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https.expect_send().never();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");
        let account = AccountFacts {
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        };
        let key = KeySource::Given(KeyText::new(KEY).expect("a key"));
        let days = DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        };

        let bill = batch.provider().expect("a provider part").read_bill(
            &key_facts(),
            &account,
            &key,
            &[],
            &days,
        );

        let Ok(Err(ReadError::NeedsUser(need))) = bill else {
            panic!("a need: {bill:?}");
        };
        assert!(
            matches!(&need.action, NeedAction::AskKey(ask) if ask.slot.as_deref() == Some("admin")),
            "{need:?}"
        );
    }

    /// Spec 2026-11-19-agent-plugins, Decision 5: a key the user gave in
    /// OTTY reaches the part only as a handle; the host fills it into the
    /// request.
    #[test]
    fn a_key_given_in_otty_reaches_the_part_as_a_handle() {
        let given = KeyText::new(KEY).expect("a filled key");
        let key = KeySource::Given(given);

        assert_eq!(money_left_with(&key, &template_env()), 12.5);
    }
}
