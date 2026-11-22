use otty_app_domain::plugins::{PluginNotes, WaitingUpdate};
use otty_plugin_host::fixtures::paused_health;
use otty_plugin_host::{PartHealth, PluginLog};
use otty_plugin_manifest::PluginVersion;
use otty_plugin_releases::InstallPlan;
use otty_plugin_releases::fixtures::{
    example_manifest, install_plan, pending_update, report, with_part_on,
};

use super::*;
use crate::model::{PageViewModel, PluginRowViewModel};
use crate::testing::{
    agent, fork, official_example, provider, released_example,
};
use crate::types::RunningPlugin;

fn open_state(plugins: Vec<ListedPlugin>) -> PluginsState {
    let mut state =
        PluginsState::new(plugins).with_home(PathBuf::from("/home/user"));
    let _ = reduce(&mut state, PluginsIntent::Open(PluginsPlace::List));

    state
}

fn rows(state: &PluginsState) -> Vec<PluginRowViewModel> {
    match PageViewModel::from_state(state) {
        Some(PageViewModel::List { rows, .. }) => rows,
        other => panic!("the list: {other:?}"),
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

/// The page opens from Settings over everything and takes the keyboard;
/// Esc goes back from a dialog of the page, then closes it.
#[test]
fn the_page_opens_and_esc_goes_back_then_closes() {
    let mut state = PluginsState::new(Vec::new());

    let opened = reduce(&mut state, PluginsIntent::Open(PluginsPlace::List));
    let added = reduce(&mut state, PluginsIntent::AddPlugin);
    let back = reduce(&mut state, PluginsIntent::Escape);
    let on_list = *state.screen() == Screen::List;
    let closed = reduce(&mut state, PluginsIntent::Escape);

    assert_eq!(opened, [PluginsOutcome::Opened]);
    assert_eq!(added, [PluginsOutcome::FocusField(URL_FIELD)]);
    assert!(back.is_empty());
    assert!(on_list);
    assert_eq!(closed, [PluginsOutcome::Closed]);
    assert!(!state.is_open());
}

/// Spec "What the UI shows": Add plugin of the need opens the page with
/// the URL field empty.
#[test]
fn add_plugin_of_a_need_opens_the_empty_url_field() {
    let mut state = PluginsState::new(Vec::new());

    let _ = reduce(&mut state, PluginsIntent::Open(PluginsPlace::AddPlugin));

    assert_eq!(*state.screen(), Screen::AddUrl { url: String::new() });
}

/// "Read by" opens the page at the plugin's row.
#[test]
fn read_by_opens_the_page_at_its_plugin() {
    let mut state = PluginsState::new(vec![official_example()]);

    let _ = reduce(
        &mut state,
        PluginsIntent::Open(PluginsPlace::Plugin(String::from("example"))),
    );

    assert!(rows(&state)[0].is_highlighted);
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
    assert!(matches!(state.screen(), Screen::AddUrl { .. }));
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

    assert_eq!(*state.screen(), Screen::List);
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
    let rows = rows(&state);
    assert!(!rows[0].parts[0].is_on);
    assert!(rows[0].parts[1].is_on);
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
fn a_failed_switch_says_why_on_the_list() {
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
    let paused = rows(&state)[0].parts[0].is_paused;

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
    let row = rows(&state)[0].clone();
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
    assert!(rows(&state)[0].waiting.is_some());
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

    assert_eq!(opened, [PluginsOutcome::Opened]);
    assert_eq!(
        applied,
        [PluginsOutcome::ApplyUpdate {
            update: Box::new(update),
        }]
    );
    assert_eq!(*state.screen(), Screen::List);
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

    let row = rows(&state)[0].clone();
    assert!(row.is_news_failure);
    assert_eq!(
        row.news.as_deref(),
        Some(otty_app_i18n::t(
            otty_app_i18n::Key::PluginsErrorRateLimited
        ))
    );
}

/// The menu "⋯" of a local plugin reloads it; a reload that asks for
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
    assert_eq!(*state.screen(), Screen::List);
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
