use otty_app_domain::plugins::{PluginNotes, WaitingUpdate};
use otty_plugin_host::fixtures::paused_health;
use otty_plugin_host::{PartHealth, PluginLog};
use otty_plugin_manifest::PluginVersion;
use otty_plugin_releases::InstallPlan;
use otty_plugin_releases::fixtures::{
    example_manifest, install_plan, pending_update, report, with_part_on,
};

use super::*;
use crate::model::{MainViewModel, PluginPageViewModel};
use crate::testing::{
    agent, fork, official_example, provider, released_example,
};
use crate::types::RunningPlugin;

/// Plugins mode with `plugins` installed and the plugin `example`
/// selected.
fn open_state(plugins: Vec<ListedPlugin>) -> PluginsState {
    let mut state =
        PluginsState::new(plugins).with_home(PathBuf::from("/home/user"));
    let _ = reduce(&mut state, PluginsIntent::Select(String::from("example")));

    state
}

/// The page of the selected plugin.
fn page(state: &PluginsState) -> PluginPageViewModel {
    match MainViewModel::from_state(state, std::time::SystemTime::now()) {
        MainViewModel::Plugin(page) => *page,
        other => panic!("the page of a plugin: {other:?}"),
    }
}

fn fork_plan() -> InstallPlan {
    let fork = fork();
    install_plan(
        &example_manifest("0.1.0")
            .replace("id = \"example\"", "id = \"example-fork\"")
            .replace("name = \"Example\"", "name = \"Example Fork\""),
        fork.installed.source().clone(),
    )
}

fn waiting_example() -> (ListedPlugin, otty_plugin_releases::PendingUpdate) {
    let plugin = released_example();
    let newer = example_manifest("0.2.0").replace(
        "http = [\"api.example.com\"]",
        "http = [\"api.example.com\", \"x.example.com\"]",
    );
    let update = pending_update(&plugin.installed, &newer);

    (plugin, update)
}

/// Issue 30: Manage plugins… of Agents mode opens the mode with nothing
/// selected and no dialog, whatever was selected before.
#[test]
fn open_at_the_list_selects_nothing() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(&mut state, PluginsIntent::AddPlugin);

    let outcomes = reduce(&mut state, PluginsIntent::Open(PluginsPlace::List));

    assert!(outcomes.is_empty());
    assert_eq!(state.selected(), None);
    assert_eq!(state.dialog(), None);
}

/// Issue 30: "Read by" of Agents mode opens the mode with its plugin
/// selected; a plugin that is not installed selects nothing.
#[test]
fn open_from_agents_selects_the_plugin() {
    let mut state = PluginsState::new(vec![official_example(), fork()]);

    let _ = reduce(
        &mut state,
        PluginsIntent::Open(PluginsPlace::Plugin(String::from("example-fork"))),
    );
    let fork_selected = state.selected().map(String::from);
    let _ = reduce(
        &mut state,
        PluginsIntent::Open(PluginsPlace::Plugin(String::from("gone"))),
    );

    assert_eq!(fork_selected.as_deref(), Some("example-fork"));
    assert_eq!(state.selected(), None);
}

/// Spec "What the UI shows": Add plugin of the need opens the URL field,
/// empty, with the keyboard in it.
#[test]
fn add_plugin_of_a_need_opens_the_empty_url_field() {
    let mut state = PluginsState::new(Vec::new());

    let outcomes =
        reduce(&mut state, PluginsIntent::Open(PluginsPlace::AddPlugin));

    assert_eq!(outcomes, [PluginsOutcome::FocusField(URL_FIELD)]);
    assert_eq!(state.dialog(), Some(&Dialog::AddUrl { url: String::new() }));
}

/// Esc or Cancel closes a dialog and keeps the selection.
#[test]
fn back_closes_the_dialog_and_keeps_the_selection() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(&mut state, PluginsIntent::AddFromFolder);

    let outcomes = reduce(&mut state, PluginsIntent::Back);

    assert!(outcomes.is_empty());
    assert_eq!(state.dialog(), None);
    assert_eq!(state.selected(), Some("example"));
}

/// A press on a plugin of the left panel selects it; an id that is not
/// installed selects nothing.
#[test]
fn a_press_selects_the_plugin() {
    let mut state = PluginsState::new(vec![official_example(), fork()]);

    let _ = reduce(&mut state, PluginsIntent::Select(String::from("example")));
    let example = state.selected().map(String::from);
    let _ = reduce(&mut state, PluginsIntent::Select(String::from("gone")));

    assert_eq!(example.as_deref(), Some("example"));
    assert_eq!(state.selected(), None);
}

#[test]
fn the_search_keeps_what_was_typed() {
    let mut state = PluginsState::new(vec![official_example()]);

    let _ =
        reduce(&mut state, PluginsIntent::SearchChanged(String::from("Ex")));

    assert_eq!(state.search(), "Ex");
}

/// Issue 30: when the selected plugin is removed, nothing is selected
/// and the main area says what to do.
#[test]
fn removing_the_selected_plugin_selects_nothing() {
    let mut state = open_state(vec![released_example(), fork()]);
    let _ = reduce(
        &mut state,
        PluginsIntent::AskRemove(String::from("example")),
    );
    let _ = reduce(&mut state, PluginsIntent::Remove);

    let _ = reduce(&mut state, PluginsIntent::Changed(Ok(vec![fork()])));

    assert_eq!(state.selected(), None);
    assert!(matches!(
        MainViewModel::from_state(&state, std::time::SystemTime::now()),
        MainViewModel::Hint { .. }
    ));
}

/// A plugin removed elsewhere, as by another change, drops the selection
/// too; another plugin that stays keeps it.
#[test]
fn a_change_keeps_the_selection_of_a_plugin_that_stays() {
    let mut state = open_state(vec![official_example(), fork()]);

    let _ = reduce(
        &mut state,
        PluginsIntent::Changed(Ok(vec![official_example()])),
    );

    assert_eq!(state.selected(), Some("example"));
}

/// A finished install selects the plugin it installed.
#[test]
fn a_finished_install_selects_the_new_plugin() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(&mut state, PluginsIntent::Found(Ok(Box::new(fork_plan()))));
    let _ = reduce(&mut state, PluginsIntent::Install);

    let _ = reduce(
        &mut state,
        PluginsIntent::Changed(Ok(vec![official_example(), fork()])),
    );

    assert_eq!(state.selected(), Some("example-fork"));
    assert_eq!(state.dialog(), None);
}

#[test]
fn next_under_a_link_looks_for_its_release() {
    let mut state = open_state(Vec::new());
    let _ = reduce(&mut state, PluginsIntent::AddPlugin);
    let _ = reduce(
        &mut state,
        PluginsIntent::UrlChanged(String::from(
            " https://github.com/someone/otty-plugin-example ",
        )),
    );

    let outcomes = reduce(&mut state, PluginsIntent::Find);
    let again = reduce(&mut state, PluginsIntent::Find);

    assert_eq!(
        outcomes,
        [PluginsOutcome::FindRelease(String::from(
            "https://github.com/someone/otty-plugin-example"
        ))]
    );
    assert!(again.is_empty(), "the first look is on its way");
    assert!(state.is_busy());
}

#[test]
fn a_folder_with_a_tilde_is_in_the_home_folder() {
    let mut state = open_state(Vec::new());
    let _ = reduce(&mut state, PluginsIntent::AddFromFolder);
    let _ = reduce(
        &mut state,
        PluginsIntent::FolderChanged(String::from("~/work/plugin/target")),
    );

    let outcomes = reduce(&mut state, PluginsIntent::Find);

    assert_eq!(
        outcomes,
        [PluginsOutcome::FindLocal(PathBuf::from(
            "/home/user/work/plugin/target"
        ))]
    );
}

/// Errors in plain words: a repository whose releases all need a newer
/// OTTY says so, and the link stays to fix.
#[test]
fn a_release_that_needs_a_newer_otty_says_so() {
    let mut state = open_state(Vec::new());
    let _ = reduce(&mut state, PluginsIntent::AddPlugin);
    let _ = reduce(&mut state, PluginsIntent::UrlChanged(String::from("x")));
    let _ = reduce(&mut state, PluginsIntent::Find);

    let _ = reduce(
        &mut state,
        PluginsIntent::Found(Err(InstallError::NeedsNewerOtty(String::from(
            "Needs a newer OTTY (agents.agent contract 0.2)",
        )))),
    );

    assert!(!state.is_busy());
    assert_eq!(
        state.error(),
        Some(
            crate::words::needs_newer_otty(
                "Needs a newer OTTY (agents.agent contract 0.2)"
            )
            .as_str()
        )
    );
    assert!(matches!(state.dialog(), Some(Dialog::AddUrl { .. })));
}

/// Issue 29: a plugin that serves what another plugin serves installs as
/// it is: nothing is asked, and Install installs the plan.
#[test]
fn install_installs_the_plan_with_nothing_asked() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(&mut state, PluginsIntent::AddPlugin);
    let _ = reduce(&mut state, PluginsIntent::Found(Ok(Box::new(fork_plan()))));

    let outcomes = reduce(&mut state, PluginsIntent::Install);

    assert_eq!(
        outcomes,
        [PluginsOutcome::Install {
            plan: Box::new(fork_plan()),
        }]
    );
}

/// A finished install goes back to the list and the binary loads the
/// parts that are on and tells Agents mode the notes.
#[test]
fn a_finished_install_loads_the_parts_that_are_on() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(&mut state, PluginsIntent::Found(Ok(Box::new(fork_plan()))));
    let _ = reduce(&mut state, PluginsIntent::Install);
    let mut example = official_example();
    example.installed = with_part_on(example.installed, &provider(), false);
    let mut installed_fork = fork();
    installed_fork.installed =
        with_part_on(installed_fork.installed, &agent(), false);
    let now = vec![example.clone(), installed_fork.clone()];

    let outcomes = reduce(&mut state, PluginsIntent::Changed(Ok(now)));

    assert_eq!(state.dialog(), None);
    assert!(!state.is_busy());
    let [
        PluginsOutcome::LoadParts(parts),
        PluginsOutcome::Notes(notes),
    ] = outcomes.as_slice()
    else {
        panic!("the parts and the notes: {outcomes:?}");
    };
    assert_eq!(parts[0].on, [agent()]);
    assert_eq!(parts[1].on, [provider()]);
    assert_eq!(notes.off_parts.len(), 2);
}

/// Issue 09: turning one part off asks for that part only, and the
/// list after it has the other part on.
#[test]
fn turning_one_part_off_leaves_the_others_on() {
    let mut state = open_state(vec![official_example()]);

    let outcomes = reduce(
        &mut state,
        PluginsIntent::SetPartOn {
            plugin: String::from("example"),
            part: agent(),
            on: false,
        },
    );
    let mut example = official_example();
    example.installed = with_part_on(example.installed, &agent(), false);
    let _ = reduce(&mut state, PluginsIntent::Changed(Ok(vec![example])));

    assert_eq!(
        outcomes,
        [PluginsOutcome::SetPartOn {
            plugin: String::from("example"),
            part: agent(),
            on: false,
        }]
    );
    let page = page(&state);
    assert!(!page.parts[0].is_on);
    assert!(page.parts[1].is_on);
}

/// A change that leaves the plugins as they were loads nothing again.
#[test]
fn an_unchanged_list_loads_no_parts() {
    let mut state = open_state(vec![official_example()]);

    let outcomes = reduce(
        &mut state,
        PluginsIntent::Changed(Ok(vec![official_example()])),
    );

    assert!(matches!(outcomes.as_slice(), [PluginsOutcome::Notes(_)]));
}

#[test]
fn a_failed_switch_says_why_on_the_page() {
    let mut state = open_state(vec![official_example()]);
    let _ = reduce(
        &mut state,
        PluginsIntent::SetPartOn {
            plugin: String::from("example"),
            part: agent(),
            on: false,
        },
    );

    let outcomes = reduce(
        &mut state,
        PluginsIntent::Changed(Err(InstallError::Disk(String::from(
            "OTTY could not write plugins.json: disk full",
        )))),
    );

    assert!(outcomes.is_empty());
    assert_eq!(
        state.error(),
        Some("OTTY could not write plugins.json: disk full")
    );
}

/// Turn on of the need "<Part> of the <Plugin> plugin is off".
#[test]
fn turn_on_of_a_need_turns_its_part_on() {
    let mut state = PluginsState::new(vec![released_example()]);
    let off = otty_app_domain::plugins::OffPart {
        plugin: String::from("example"),
        plugin_name: String::from("Example"),
        version: String::from("0.1.0"),
        side: PartSide::Provider,
        part: String::from("example-cloud"),
        part_name: String::from("Example Cloud"),
        hosts: vec![String::from("example.com")],
    };

    let outcomes = reduce(&mut state, PluginsIntent::TurnOn(off));

    assert_eq!(
        outcomes,
        [PluginsOutcome::SetPartOn {
            plugin: String::from("example"),
            part: provider(),
            on: true,
        }]
    );
}

/// Try again on a paused part starts it again and Agents mode reads
/// again (issue 07: `PartHealth::try_again`).
#[test]
fn try_again_starts_a_paused_part() {
    let mut state = open_state(vec![official_example()]);
    let health = paused_health();
    let _ = reduce(
        &mut state,
        PluginsIntent::Running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), health.clone()),
                (provider(), PartHealth::default()),
            ],
        }]),
    );
    let paused = page(&state).parts[0].is_paused;

    let outcomes = reduce(
        &mut state,
        PluginsIntent::TryAgain {
            plugin: String::from("example"),
            part: agent(),
        },
    );

    assert!(paused);
    assert!(!health.is_paused());
    assert_eq!(outcomes, [PluginsOutcome::ReadAgain]);
}

/// Issue 09: an automatic update shows "Updated to <version>" and loads
/// the new parts; one that waits shows on the row and in the notes.
#[test]
fn the_checks_note_updates_and_waiting_updates() {
    let (plugin, update) = waiting_example();
    let mut state = open_state(vec![plugin.clone(), official_example()]);
    let updated = report(
        "example",
        Ok(UpdateOutcome::Updated {
            from: PluginVersion::parse("0.1.0").expect("version"),
            to: PluginVersion::parse("0.2.0").expect("version"),
        }),
        Some(("v0.3.0", "Needs a newer OTTY (agents.agent contract 0.2)")),
    );

    let _ = reduce(
        &mut state,
        PluginsIntent::Checked {
            reports: vec![updated],
            plugins: vec![plugin.clone()],
            asked: false,
        },
    );
    let row = page(&state);
    let waits = report(
        "example",
        Ok(UpdateOutcome::Waiting(Box::new(update))),
        None,
    );
    let outcomes = reduce(
        &mut state,
        PluginsIntent::Checked {
            reports: vec![waits],
            plugins: vec![plugin],
            asked: false,
        },
    );

    let updated_to = otty_app_i18n::filled(
        otty_app_i18n::Key::TplPluginsUpdatedTo,
        &[("version", "0.2.0")],
    );
    assert!(row.news.expect("news").starts_with(&updated_to));
    assert_eq!(
        outcomes,
        [PluginsOutcome::Notes(PluginNotes {
            off_parts: Vec::new(),
            waiting_updates: vec![WaitingUpdate {
                plugin: String::from("example"),
                name: String::from("Example"),
                from: String::from("0.1.0"),
                to: String::from("0.2.0"),
            }],
        })]
    );
    assert!(page(&state).waiting.is_some());
}

/// Review of the need "Plugin update needs your OK" opens the update
/// dialog; Update installs it.
#[test]
fn review_opens_the_update_dialog_and_update_applies_it() {
    let (plugin, update) = waiting_example();
    let mut state = PluginsState::new(vec![plugin.clone()]);
    state.set_news("example", PluginNews::Waiting(Box::new(update.clone())));

    let opened = reduce(
        &mut state,
        PluginsIntent::Open(PluginsPlace::Review(String::from("example"))),
    );
    let applied = reduce(&mut state, PluginsIntent::ApplyUpdate);
    let _ = reduce(&mut state, PluginsIntent::Changed(Ok(vec![plugin])));

    assert!(opened.is_empty());
    assert_eq!(state.selected(), Some("example"));
    assert_eq!(
        applied,
        [PluginsOutcome::ApplyUpdate {
            update: Box::new(update),
        }]
    );
    assert_eq!(state.dialog(), None);
    assert_eq!(
        state.news_of("example"),
        Some(&PluginNews::Updated(String::from("0.2.0")))
    );
}

/// Check for updates of a plugin that has nothing newer says "Up to
/// date"; the daily check leaves the row as it is.
#[test]
fn only_a_check_the_user_asked_for_says_up_to_date() {
    let mut state = open_state(vec![released_example()]);
    let up_to_date = || report("example", Ok(UpdateOutcome::UpToDate), None);

    let _ = reduce(
        &mut state,
        PluginsIntent::Checked {
            reports: vec![up_to_date()],
            plugins: vec![released_example()],
            asked: false,
        },
    );
    let daily = state.news_of("example").cloned();
    let _ = reduce(
        &mut state,
        PluginsIntent::Checked {
            reports: vec![up_to_date()],
            plugins: vec![released_example()],
            asked: true,
        },
    );

    assert_eq!(daily, None);
    assert_eq!(state.news_of("example"), Some(&PluginNews::UpToDate));
}

#[test]
fn a_failed_check_says_why_on_the_row() {
    let mut state = open_state(vec![released_example()]);

    let _ = reduce(
        &mut state,
        PluginsIntent::Checked {
            reports: vec![report(
                "example",
                Err(InstallError::RateLimited),
                None,
            )],
            plugins: vec![released_example()],
            asked: true,
        },
    );

    let row = page(&state);
    assert!(row.is_news_failure);
    assert_eq!(
        row.news.as_deref(),
        Some(otty_app_i18n::t(
            otty_app_i18n::Key::PluginsErrorRateLimited
        ))
    );
}

/// Reload of a local plugin reloads it; a reload that asks for
/// more waits for the user like an update.
#[test]
fn reload_of_a_local_plugin_that_asks_for_more_waits() {
    let mut state = open_state(vec![crate::testing::local_example()]);
    let (_, update) = waiting_example();

    let asked =
        reduce(&mut state, PluginsIntent::Reload(String::from("example")));
    let _ = reduce(
        &mut state,
        PluginsIntent::Reloaded {
            plugin: String::from("example"),
            outcome: Ok(UpdateOutcome::Waiting(Box::new(update))),
            plugins: vec![crate::testing::local_example()],
        },
    );

    assert_eq!(asked, [PluginsOutcome::Reload(String::from("example"))]);
    assert!(state.waiting_update("example").is_some());
}

#[test]
fn remove_asks_first_then_removes() {
    let mut state = open_state(vec![released_example()]);

    let asked = reduce(
        &mut state,
        PluginsIntent::AskRemove(String::from("example")),
    );
    let removed = reduce(&mut state, PluginsIntent::Remove);
    let after = reduce(&mut state, PluginsIntent::Changed(Ok(Vec::new())));

    assert!(asked.is_empty());
    assert_eq!(removed, [PluginsOutcome::Remove(String::from("example"))]);
    assert_eq!(after[0], PluginsOutcome::LoadParts(Vec::new()));
    assert_eq!(state.dialog(), None);
}

#[test]
fn open_repository_opens_the_repository_of_the_manifest() {
    let mut state = open_state(vec![official_example()]);

    let outcomes = reduce(
        &mut state,
        PluginsIntent::OpenRepository(String::from("example")),
    );

    assert_eq!(
        outcomes,
        [PluginsOutcome::OpenPage(String::from(
            "https://github.com/otty-shell/otty-plugin-example"
        ))]
    );
}

#[test]
fn the_daily_timer_asks_for_the_check_when_it_is_due() {
    let mut state = PluginsState::new(Vec::new());

    let outcomes = reduce(&mut state, PluginsIntent::UpdateTick);

    assert_eq!(outcomes, [PluginsOutcome::CheckUpdatesIfDue]);
}

/// Issue 19: Copy report of the last failed call of a part puts its
/// report on the clipboard.
#[test]
fn copy_report_of_a_part_copies_its_report() {
    let mut state = open_state(vec![official_example()]);
    let report = String::from("The plugin Example 0.1.0 could not read.");

    let outcomes =
        reduce(&mut state, PluginsIntent::CopyReport(report.clone()));

    assert_eq!(outcomes, [PluginsOutcome::CopyText(report)]);
}
