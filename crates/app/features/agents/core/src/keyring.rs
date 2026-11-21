//! The system keyring (spec 2026-11-19, Decision 8): a plugin part may
//! declare entries of the system keyring where its agent keeps keys, and
//! read them through the plugin host. The parts read nothing there before
//! the user allowed it once in the keyring dialog; the allowance is saved
//! in `agents.json`. The user is asked only while a part that is on
//! declares an entry.
//!
//! A read may make the system ask the user to unlock the keyring, so the
//! plugins are let read it only once Agents mode is on screen: at the first
//! Enter of a run that finds the allowance, or at once after Allow. The
//! profiles are then read again, and the keys the parts find in the
//! keyring are listed with the others.

use otty_agents::Registry;

use super::event::AgentsOutcome;
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;

/// The names of the parts of `registry` that declare an entry of the
/// system keyring, agents first, such as "Coder, Example Cloud".
pub(crate) fn keyring_part_names(registry: &Registry) -> Vec<&str> {
    let agents = registry
        .agents()
        .iter()
        .filter(|entry| !entry.keyring_services().is_empty())
        .map(|entry| entry.name());
    let providers = registry
        .providers()
        .iter()
        .filter(|entry| !entry.keyring_services().is_empty())
        .map(|entry| entry.name());

    agents.chain(providers).collect()
}

/// The services of the keyring entries the parts of `registry` declare,
/// each once, in their order.
pub(crate) fn keyring_services(registry: &Registry) -> Vec<&str> {
    let agents = registry
        .agents()
        .iter()
        .flat_map(|entry| entry.keyring_services());
    let providers = registry
        .providers()
        .iter()
        .flat_map(|entry| entry.keyring_services());

    let mut services: Vec<&str> = Vec::new();
    for service in agents.chain(providers) {
        if !services.contains(&service.as_str()) {
            services.push(service);
        }
    }

    services
}

/// Let the plugins read the keyring when the user allowed it, Agents mode
/// is on screen, a part that is on declares an entry, and they were not
/// let yet in this run.
pub(crate) fn ask(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let is_allowed = state
        .profiles()
        .saved()
        .is_some_and(|saved| saved.is_keyring_allowed());
    let is_declared = !keyring_part_names(state.registry()).is_empty();
    if !state.is_active()
        || !is_allowed
        || !is_declared
        || state.is_keyring_given()
    {
        return Vec::new();
    }

    give(state)
}

/// Open the keyring dialog, which asks the user to allow the read.
pub(crate) fn open_dialog(state: &mut AgentsState) {
    state.set_keyring_dialog_open(true);
}

/// Allow of the keyring dialog: save the allowance in `agents.json` and
/// let the plugins read the keyring at once.
pub(crate) fn allow(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state.set_keyring_dialog_open(false);
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().with_keyring_allowed();
    state.profiles_mut().apply_saved(changed.clone());

    let mut outcomes = vec![save(changed)];
    outcomes.extend(give(state));
    outcomes
}

/// Not now of the keyring dialog: close it; nothing is read or saved.
pub(crate) fn not_now(state: &mut AgentsState) {
    state.set_keyring_dialog_open(false);
}

/// Let the plugins read the keyring for the rest of the run, and read the
/// profiles again so the parts find the keys there.
fn give(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state.give_keyring();

    let mut outcomes = vec![AgentsOutcome::AllowPluginKeyring];
    if state.profiles_mut().start_reload() {
        outcomes.push(AgentsOutcome::LoadProfiles);
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_agents::{AgentEntry, AgentId, SavedProfiles};
    use otty_plugin_host::PluginPart;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::account_needs;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, connected, env, profiles_loaded_from, save as saved,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    fn allowed() -> SavedProfiles {
        report_shown().with_keyring_allowed()
    }

    /// The registry of the tests with a made-up agent part that declares
    /// the keyring entry of the service `example-keyring`.
    fn registry_with_keyring_part() -> Arc<Registry> {
        let test = crate::testing::registry();
        let part = PluginPart::new("example", "Example", "0.1.0", "", "keeper");
        let keeper = AgentEntry::new(
            AgentId::new("keeper"),
            "Keeper",
            "keeper",
            "Example",
            part,
        )
        .with_keyring_services(vec![String::from("example-keyring")]);
        let mut agents = test.agents().to_vec();
        agents.push(keeper);

        Arc::new(Registry::new(agents, test.providers().to_vec()))
    }

    /// Agents mode on screen with `registry` and `saved` read.
    fn on_screen(registry: Arc<Registry>, saved: SavedProfiles) -> AgentsState {
        let mut state = AgentsState::new(agents(), registry, env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            profiles_loaded_from(&[(claude(), connected())], saved),
        );
        state
    }

    fn keyring_given(outcomes: &[AgentsOutcome]) -> usize {
        outcomes
            .iter()
            .filter(|outcome| {
                matches!(outcome, AgentsOutcome::AllowPluginKeyring)
            })
            .count()
    }

    fn keyring_need_actions(state: &AgentsState) -> Vec<Option<NeedAction>> {
        account_needs(state)
            .into_iter()
            .map(|need| need.fix.and_then(|fix| fix.action))
            .filter(|action| matches!(action, Some(NeedAction::AllowKeyring)))
            .collect()
    }

    /// Issue 17: with only parts that declare no keyring entry, as the
    /// official plugins, the user is never asked about the keyring and the
    /// plugins are never let read it.
    #[test]
    fn with_no_part_that_declares_an_entry_the_keyring_is_never_asked_about() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());

        let entered = reduce(&mut state, AgentsIntent::Enter);
        let loaded = reduce(
            &mut state,
            profiles_loaded_from(&[(claude(), connected())], allowed()),
        );

        assert!(keyring_need_actions(&state).is_empty());
        assert_eq!(keyring_given(&entered) + keyring_given(&loaded), 0);
    }

    #[test]
    fn nothing_is_given_to_the_plugins_before_the_user_allows_it() {
        let mut state =
            AgentsState::new(agents(), registry_with_keyring_part(), env());

        let entered = reduce(&mut state, AgentsIntent::Enter);
        let loaded = reduce(
            &mut state,
            profiles_loaded_from(&[(claude(), connected())], report_shown()),
        );

        assert_eq!(keyring_given(&entered) + keyring_given(&loaded), 0);
        assert_eq!(
            keyring_need_actions(&state),
            [Some(NeedAction::AllowKeyring)]
        );
    }

    /// The need's button opens the dialog, and Allow saves the allowance,
    /// lets the plugins read the keyring, reads the profiles again and
    /// the need is gone.
    #[test]
    fn allow_saves_the_allowance_gives_the_keyring_and_the_need_goes() {
        let mut state = on_screen(registry_with_keyring_part(), report_shown());

        let opened =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AllowKeyring));
        let is_open = state.is_keyring_dialog_open();
        let allowed_now = reduce(&mut state, AgentsIntent::AllowKeyring);

        assert!(opened.is_empty(), "{opened:?}");
        assert!(is_open, "the button opens the dialog");
        assert_eq!(
            allowed_now,
            vec![
                saved(allowed()),
                AgentsOutcome::AllowPluginKeyring,
                AgentsOutcome::LoadProfiles
            ]
        );
        assert!(!state.is_keyring_dialog_open());
        assert!(keyring_need_actions(&state).is_empty());
    }

    #[test]
    fn not_now_closes_the_dialog_and_the_need_stays() {
        let mut state = on_screen(registry_with_keyring_part(), report_shown());
        let _ =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AllowKeyring));

        let outcomes = reduce(&mut state, AgentsIntent::KeyringNotNow);

        assert!(outcomes.is_empty(), "{outcomes:?}");
        assert!(!state.is_keyring_dialog_open());
        assert_eq!(
            keyring_need_actions(&state),
            [Some(NeedAction::AllowKeyring)]
        );
    }

    /// The system may ask to unlock the keyring, so the plugins are let
    /// read it only once the mode is on screen, and once in a run.
    #[test]
    fn an_allowed_keyring_is_given_on_screen_once_in_a_run() {
        let mut state =
            AgentsState::new(agents(), registry_with_keyring_part(), env());
        let loaded =
            || profiles_loaded_from(&[(claude(), connected())], allowed());

        let away = reduce(&mut state, loaded());
        let entered = reduce(&mut state, AgentsIntent::Enter);
        let loaded_again = reduce(&mut state, loaded());
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = reduce(&mut state, loaded());

        assert_eq!(keyring_given(&away), 0, "{away:?}");
        assert_eq!(keyring_given(&entered), 1, "{entered:?}");
        assert_eq!(keyring_given(&loaded_again), 0, "{loaded_again:?}");
        assert_eq!(keyring_given(&refreshed), 0, "{refreshed:?}");
    }

    /// `agents.json` could not be read, so the allowance could not be
    /// saved: the user is not asked.
    #[test]
    fn with_an_unread_agents_json_the_keyring_is_not_asked_about() {
        let mut state =
            AgentsState::new(agents(), registry_with_keyring_part(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let AgentsIntent::ProfilesLoaded(loaded) =
            profiles_loaded_from(&[(claude(), connected())], report_shown())
        else {
            unreachable!("a profiles read is made");
        };
        let _ = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(crate::types::LoadedProfiles {
                saved: None,
                ..loaded
            }),
        );

        assert!(keyring_need_actions(&state).is_empty());
    }

    #[test]
    fn the_parts_and_services_that_declare_an_entry_are_named() {
        let registry = registry_with_keyring_part();

        assert_eq!(keyring_part_names(&registry), ["Keeper"]);
        assert_eq!(keyring_services(&registry), ["example-keyring"]);
        assert!(keyring_part_names(&crate::testing::registry()).is_empty());
    }
}
