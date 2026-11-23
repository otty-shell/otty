//! The provider contract: the bindings of each version the host keeps,
//! the types of the newest version, and the calls into a provider part.
//!
//! A new version goes into its own module next to the old ones, such as
//! `v0_2`; the types below then come from it, and the old version's arm
//! of [`ProviderCalls`] converts its types to them with `From`.

pub mod v0_1;

use std::time::Instant;

use wasmtime::Store;
use wasmtime::component::InstancePre;

use crate::agents::agent::ReadError;
/// The newest provider contract's types, which the rest of OTTY uses.
pub use crate::agents::provider::v0_1::exports::otty::agents_provider::provider::{
    Account, AccountKind, AccountNumbers, Bill, Credits, DayCost, DayRange,
    ExtraUsage, Found, KeyLimit, LimitWindow, LoginFolder, Money,
};
use crate::agents::provider::v0_1::exports::otty::agents_provider::provider::{
    Guest, GuestIndices,
};
use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::ProviderVersion;
use crate::keys::{KeyHandle, KeySource};
use crate::operations::v0_1::otty::host::keys::GivenKey;
use crate::operations::v0_1::otty::host::types::Command;
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
/// [`CallBatch`].
///
/// Each call gives the part its login folders: the folders `accounts` is
/// asked about, or the `found-in` of the account `numbers` and
/// `read-bill` read. Inside them, for that call only, the part reads what
/// the `logins` of its manifest declare for the agent part of the folder
/// (spec 2026-11-23-accounts-apart-from-profiles, Decision 5); the caller
/// gives only folders of profiles of the part's own plugin. The keys the
/// user gave for an account after a need of the part asked for them go
/// with their slot, each as a handle for that call only, never the key.
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

    /// The accounts the part finds in `folders`, and its needs that
    /// belong to no account.
    pub fn accounts(
        &mut self,
        folders: &[LoginFolder],
    ) -> Result<Found, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(folders, &[], |guest, store, _| {
            guest.call_accounts(store, &provider, folders)
        })
    }

    /// The numbers of `account` right now, read any way the part can in
    /// the folders it was found in, with the keys of `given`, each with
    /// its slot. A reading that ran a program or a hidden terminal is
    /// slow: it runs at most once in the five minutes of the limits read
    /// per part and account id, and the answer of the last reading is
    /// given within that time.
    pub fn numbers(
        &mut self,
        account: &Account,
        given: &[(String, KeySource)],
    ) -> Result<Result<AccountNumbers, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();
        let plugin = self.batch.plugin();
        let pace = PaceKey::new(self.batch.part(), &account.id);
        let started_at = Instant::now();
        if let Some(answer) = plugin.pacing().recent(&pace, started_at) {
            return Ok(answer);
        }

        let (answer, is_slow) = self.call_with_keys(
            &account.found_in,
            given,
            |guest, store, given| {
                let answer = guest.call_numbers(
                    &mut *store,
                    &provider,
                    account,
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

    /// The bill of the organization `account` over `days`, read in the
    /// folders it was found in with the keys of `given`, each with its
    /// slot.
    pub fn read_bill(
        &mut self,
        account: &Account,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<Result<Bill, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(&account.found_in, given, |guest, store, given| {
            guest.call_read_bill(store, &provider, account, &given, days)
        })
    }

    /// The agent's official login command pointed at OTTY's own login
    /// folder `folder`, such as `codex login` with `CODEX_HOME` set to it
    /// (spec 2026-11-23-accounts-apart-from-profiles, Decision 6); `None`
    /// from a part that offers no "Log in with". The part only says the
    /// command: OTTY runs it in a terminal tab the user sees.
    pub fn log_in(
        &mut self,
        folder: &str,
    ) -> Result<Option<Command>, CallError> {
        let provider = self.batch.part().id().to_string();
        let exports = self.exports.clone();

        self.batch
            .call(CallKind::Quick, |store, instance| match &exports {
                ProviderExports::V0_1(indices) => {
                    let guest = indices.load(&mut *store, instance)?;

                    guest.call_log_in(store, &provider, folder)
                },
            })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract, the login folders `folders` and a handle to each key of
    /// `given` with its slot, given to the plugin for this call.
    fn call_with_keys<T>(
        &mut self,
        folders: &[LoginFolder],
        given: &[(String, KeySource)],
        call: impl FnOnce(
            &Guest,
            &mut Store<CallState>,
            Vec<GivenKey>,
        ) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();
        let folders = folders_of_call(folders);

        self.batch.call_in_folders(
            CallKind::Numbers,
            &folders,
            |store, instance| match &exports {
                ProviderExports::V0_1(indices) => {
                    let guest = indices.load(&mut *store, instance)?;
                    let mut given_keys = Vec::new();
                    for (slot, source) in given {
                        let key = KeyHandle::of(source.clone());
                        given_keys.push(GivenKey {
                            slot: slot.clone(),
                            key: store.data_mut().table().push(key)?,
                        });
                    }

                    call(&guest, store, given_keys)
                },
            },
        )
    }
}

/// `folders` as the permissions of a call name them: the agent part of
/// each profile and its folder, and OTTY's own login folders, which the
/// `logins` entry that offers `log_in` reads.
fn folders_of_call(
    folders: &[LoginFolder],
) -> Vec<otty_plugin_manifest::LoginFolder> {
    folders
        .iter()
        .map(|folder| match folder {
            LoginFolder::Profile((agent, folder)) => {
                otty_plugin_manifest::LoginFolder::new(agent, folder)
            },
            LoginFolder::OwnLogin((_, folder)) => {
                otty_plugin_manifest::LoginFolder::own(folder)
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::calls::CallKind;
    use crate::https::MockHttpsSend;
    use crate::keys::KeyText;
    use crate::operations::v0_1::otty::host::files::Host as _;
    use crate::operations::v0_1::otty::host::http::Response;
    use crate::operations::v0_1::otty::host::types::{
        Command, HostError, NeedAction,
    };
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{
        TempDir, debug_err, engine_with, load_template, template_env,
        write_file,
    };

    const KEY: &str = "ex-made-up-key-0001";

    /// The template's provider part.
    const PROVIDER: &str = "example-cloud";

    /// The template's agent part, whose profiles hold the logins its
    /// provider part reads.
    const AGENT: &str = "example-agent";

    /// A folder of a profile of the template's agent part that no glob of
    /// the template names, with the login `key` of `email` in `auth.json`
    /// and its version in `state.json`.
    fn login_folder_with(key: &str, email: &str) -> TempDir {
        let folder = TempDir::new();
        write_file(
            &folder.path().join("auth.json"),
            format!(r#"{{"api_key": "{key}", "email": "{email}"}}"#).as_bytes(),
        );
        write_file(
            &folder.path().join("state.json"),
            br#"{"last_version": "1.4.2"}"#,
        );

        folder
    }

    /// A login folder with the login [`KEY`].
    fn login_folder() -> TempDir {
        login_folder_with(KEY, "dev@example.com")
    }

    /// The folder `folder` of a profile of the agent part `agent`, as OTTY
    /// gives it to a call.
    fn profile(agent: &str, folder: &Path) -> LoginFolder {
        let folder = folder.to_str().expect("a text path").to_string();

        LoginFolder::Profile((agent.to_string(), folder))
    }

    /// What the template's provider part finds in `folders`.
    fn accounts_in(folders: &[LoginFolder]) -> Found {
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");

        batch
            .provider()
            .expect("a provider part")
            .accounts(folders)
            .expect("the call")
    }

    /// The folders an account of the template was found in.
    fn folders_of(account: &Account) -> Vec<&str> {
        account
            .found_in
            .iter()
            .map(|folder| match folder {
                LoginFolder::Profile((_, folder))
                | LoginFolder::OwnLogin((_, folder)) => folder.as_str(),
            })
            .collect()
    }

    /// An account of the template's provider part found in `folders`,
    /// with no id the part gave.
    fn account_in(kind: AccountKind, folders: &[LoginFolder]) -> Account {
        Account {
            id: "made-up".to_string(),
            name: "made-up".to_string(),
            kind,
            plan: None,
            found_in: folders.to_vec(),
            needs: Vec::new(),
        }
    }

    /// The numbers the template's provider part reads of `account`.
    /// `https` serves its requests.
    // A `ReadError` may hold a need, a record the contract can not box.
    #[allow(clippy::result_large_err)]
    fn numbers_of(
        account: &Account,
        https: MockHttpsSend,
    ) -> Result<AccountNumbers, ReadError> {
        let cache = TempDir::new();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");

        batch
            .provider()
            .expect("a provider part")
            .numbers(account, &[])
            .expect("the call")
    }

    /// An HTTP mock that answers one request for the balance, which must
    /// carry [`KEY`], with `balance` US dollars.
    fn balance_answer(balance: f64) -> MockHttpsSend {
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
            .returning(move |_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: format!(
                        r#"{{"balance": {balance}, "currency": "USD"}}"#
                    )
                    .into_bytes(),
                })
            });

        https
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8:
    /// the part finds the accounts of the logins in the folders it is
    /// given, each with its id, its name and every folder it is in.
    #[test]
    fn accounts_come_from_the_folders_of_the_call() {
        let home = login_folder();
        let work = login_folder_with("ex-made-up-key-0002", "work@example.com");
        let copy = login_folder();

        let found = accounts_in(&[
            profile(AGENT, home.path()),
            profile(AGENT, work.path()),
            profile(AGENT, copy.path()),
        ]);

        let names: Vec<&str> = found
            .accounts
            .iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(names, ["dev@example.com", "work@example.com"]);
        assert_ne!(found.accounts[0].id, found.accounts[1].id);
        assert_eq!(
            folders_of(&found.accounts[0]),
            [
                home.path().to_str().expect("a text path"),
                copy.path().to_str().expect("a text path")
            ]
        );
        assert!(found.needs.is_empty());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: in OTTY's
    /// own login folder the part finds its account as in a profile folder
    /// of the entry that offers `log_in`, and the account names that
    /// folder.
    #[test]
    fn an_account_is_found_in_an_own_login_folder() {
        let folder = login_folder();
        let text = folder.path().to_str().expect("a text path").to_string();
        let own = LoginFolder::OwnLogin(("6f0c1e2a".to_string(), text.clone()));

        let found = accounts_in(&[own]);

        assert_eq!(found.accounts.len(), 1, "{found:?}");
        assert_eq!(found.accounts[0].name, "dev@example.com");
        assert!(
            matches!(&found.accounts[0].found_in[..],
                [LoginFolder::OwnLogin((id, folder))]
                    if id == "6f0c1e2a" && *folder == text),
            "{found:?}"
        );
    }

    /// Decision 6: the part gives the agent's own login command pointed at
    /// OTTY's own login folder, which OTTY runs in a terminal tab.
    #[test]
    fn the_part_gives_its_login_command_for_an_own_folder() {
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");
        let folder = "/home/user/.local/share/otty/accounts/6f0c1e2a";

        let command = batch
            .provider()
            .expect("a provider part")
            .log_in(folder)
            .expect("the call");

        let Some(Command { program, args, env }) = command else {
            panic!("a login command");
        };
        assert_eq!(program, "example-agent");
        assert_eq!(args, ["login"]);
        assert_eq!(
            env,
            [("EXAMPLE_AGENT_HOME".to_string(), folder.to_string())]
        );
    }

    /// The part reads a login only inside a folder of an agent part its
    /// `logins` name: a folder of another agent part gives no account.
    #[test]
    fn a_folder_of_another_agent_part_gives_no_account() {
        let folder = login_folder();

        let found = accounts_in(&[profile("other-agent", folder.path())]);

        assert!(found.accounts.is_empty(), "{found:?}");
    }

    /// With no folder the part finds no account.
    #[test]
    fn no_folder_gives_no_account() {
        let found = accounts_in(&[]);

        assert!(found.accounts.is_empty());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the part
    /// reads the numbers of an account with the login in its folder,
    /// its key as a handle, also when no glob of the part names the
    /// folder.
    #[test]
    fn numbers_read_with_the_login_in_the_folders_of_the_account() {
        let folder = login_folder();
        let account =
            account_in(AccountKind::Key, &[profile(AGENT, folder.path())]);

        let numbers = numbers_of(&account, balance_answer(7.5));

        let Ok(numbers) = numbers else {
            panic!("the part's answer: {numbers:?}");
        };
        assert_eq!(numbers.left.map(|left| left.amount), Some(7.5));
    }

    /// An account found in no folder gives the part no login: it reads
    /// nothing and sends nothing.
    #[test]
    fn an_account_in_no_folder_reads_nothing() {
        let mut https = MockHttpsSend::new();
        https.expect_send().never();

        let numbers = numbers_of(&account_in(AccountKind::Key, &[]), https);

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
        let folders = folders_of_call(folders);

        batch
            .call_in_folders(CallKind::Quick, &folders, |store, _| {
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
        let own = profile(AGENT, folder.path());
        let of_another_agent = profile("other-agent", folder.path());

        let in_folder = reads_in_call(&[own], &auth);
        let with_no_folder = reads_in_call(&[], &auth);
        let in_another = reads_in_call(&[of_another_agent], &auth);

        assert_eq!(
            in_folder.0,
            Ok(r#"{"api_key":{"$key":0},"email":"dev@example.com"}"#
                .to_string())
        );
        assert_eq!(in_folder.1, Err(not_allowed(&auth)));
        assert_eq!(with_no_folder.0, Err(not_allowed(&auth)));
        assert_eq!(in_another.0, Err(not_allowed(&auth)));
    }

    /// A file of the login folder the part does not declare is refused.
    #[test]
    fn a_file_of_the_folder_not_declared_is_refused() {
        let folder = login_folder();
        let state = folder.path().join("state.json");
        let own = profile(AGENT, folder.path());

        let (with_keys, bytes) = reads_in_call(&[own], &state);

        assert_eq!(with_keys, Err(not_allowed(&state)));
        assert_eq!(bytes, Err(not_allowed(&state)));
    }

    /// The bill the template's provider part reads of an organization
    /// found in no folder, with the keys of `given`; `https` serves its
    /// requests.
    // A `ReadError` may hold a need, a record the contract can not box.
    #[allow(clippy::result_large_err)]
    fn bill_with(
        given: &[(String, KeySource)],
        https: MockHttpsSend,
    ) -> Result<Bill, ReadError> {
        let cache = TempDir::new();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider(PROVIDER))
            .expect("a listed part");
        let days = DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        };

        batch
            .provider()
            .expect("a provider part")
            .read_bill(
                &account_in(AccountKind::Organization, &[]),
                given,
                &days,
            )
            .expect("the call")
    }

    /// Issue 20: a key the user gave for a slot a need asked for reaches
    /// the next `read-bill` as a handle with that slot, and the host fills
    /// it into the part's request.
    #[test]
    fn a_given_key_reaches_the_bill_with_its_slot() {
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
        let admin = KeyText::new("ex-admin-made-up-0001").expect("a key");

        let bill =
            bill_with(&[("admin".to_string(), KeySource::Given(admin))], https);

        let Ok(bill) = bill else {
            panic!("the bill: {bill:?}");
        };
        assert_eq!(bill.days[0].cost.amount, 4.0);
    }

    /// With no key under its slot, the part asks the user for one.
    #[test]
    fn a_bill_with_no_given_key_is_a_need_of_the_part() {
        let mut https = MockHttpsSend::new();
        https.expect_send().never();

        let bill = bill_with(&[], https);

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
        assert!(
            matches!(&need.action, NeedAction::AskKey(ask) if ask.slot.as_deref() == Some("admin")),
            "{need:?}"
        );
    }
}
