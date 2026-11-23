//! What Plugins mode does with each intent: which plugin it selects and
//! which dialog it shows, what it asks `otty-app-plugins-io` to do, what it
//! tells the binary when the installed plugins change, and the
//! notifications of what Reload, Check for updates and an update did
//! (issue 35).

use std::path::{Path, PathBuf};

use otty_app_domain::plugins::PluginsPlace;
use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{InstallError, UpdateOutcome, UpdateReport};
use otty_ui_kit::components::context_menu::MenuPlace;

use crate::ctx::PluginsCtx;
use crate::event::{ChangeAnswer, PluginsIntent, PluginsOutcome};
use crate::model::failed;
use crate::notes::notes;
use crate::state::{Dialog, PluginMenu, PluginNews, PluginsState};
use crate::types::ListedPlugin;
use crate::view::{FOLDER_FIELD, URL_FIELD};
use crate::words::error_text;

/// What asked for an update check or a reload: its notifications differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    /// The daily update check: nothing to say when nothing is newer.
    DailyCheck,
    /// Check for updates: "Plugin Example is up to date" too.
    Check,
    /// Reload of a local plugin: "Plugin Example reloaded" when its
    /// version stays.
    Reload,
}

/// Reduce `intent` into state changes and outcomes; the menu of a plugin
/// opens at the pointer of `ctx`.
pub(crate) fn reduce(
    state: &mut PluginsState,
    intent: PluginsIntent,
    ctx: &PluginsCtx,
) -> Vec<PluginsOutcome> {
    // The items of the menu of a plugin send the intents of its page: the
    // menu closes and they do what the page does.
    if is_menu_item(&intent) {
        state.close_menu();
    }

    match intent {
        PluginsIntent::Open(place) => open(state, place),
        PluginsIntent::Select(plugin) => {
            state.select(Some(plugin));
            Vec::new()
        },
        PluginsIntent::OpenMenu(plugin) => {
            if state.plugin(&plugin).is_some() {
                let place = MenuPlace::new(ctx.cursor, ctx.screen_size);
                state.open_menu(PluginMenu::new(plugin, place));
            }
            Vec::new()
        },
        PluginsIntent::CloseMenu => {
            state.close_menu();
            Vec::new()
        },
        PluginsIntent::SearchChanged(text) => {
            state.set_search(text);
            Vec::new()
        },
        PluginsIntent::Back => {
            state.close_dialog();
            Vec::new()
        },
        PluginsIntent::AddPlugin => {
            state.show(Dialog::AddUrl { url: String::new() });
            vec![PluginsOutcome::FocusField(URL_FIELD)]
        },
        PluginsIntent::AddFromFolder => {
            state.show(Dialog::AddFolder {
                folder: String::new(),
            });
            vec![PluginsOutcome::FocusField(FOLDER_FIELD)]
        },
        PluginsIntent::UrlChanged(text) => {
            if let Some(Dialog::AddUrl { url }) = state.dialog_mut() {
                *url = text;
            }
            Vec::new()
        },
        PluginsIntent::FolderChanged(text) => {
            if let Some(Dialog::AddFolder { folder }) = state.dialog_mut() {
                *folder = text;
            }
            Vec::new()
        },
        PluginsIntent::Find => find(state),
        PluginsIntent::Found(found) => {
            state.set_busy(false);
            match found {
                Ok(plan) => state.show(Dialog::Install { plan }),
                Err(error) => state.set_error(Some(error_text(&error))),
            }
            Vec::new()
        },
        PluginsIntent::Install => install(state),
        PluginsIntent::SetPartOn { plugin, part, on } => {
            set_part_on(state, plugin, part, on)
        },
        PluginsIntent::TryAgain { plugin, part } => {
            try_again(state, &plugin, &part)
        },
        PluginsIntent::CopyReport(report) => {
            vec![PluginsOutcome::CopyText(report)]
        },
        PluginsIntent::CheckForUpdates(plugin) => {
            vec![PluginsOutcome::CheckUpdate(plugin)]
        },
        PluginsIntent::Reload(plugin) => vec![PluginsOutcome::Reload(plugin)],
        PluginsIntent::Review(plugin) => {
            open(state, PluginsPlace::Review(plugin))
        },
        PluginsIntent::ApplyUpdate => apply_update(state),
        PluginsIntent::ShowCanAccess { plugin, part } => {
            state.show(Dialog::CanAccess { plugin, part });
            Vec::new()
        },
        PluginsIntent::ShowLog(plugin) => {
            state.show(Dialog::Log(plugin));
            Vec::new()
        },
        PluginsIntent::OpenRepository(plugin) => repository_of(state, &plugin)
            .map(PluginsOutcome::OpenPage)
            .into_iter()
            .collect(),
        PluginsIntent::AskRemove(plugin) => {
            state.show(Dialog::Remove(plugin));
            Vec::new()
        },
        PluginsIntent::Remove => remove(state),
        PluginsIntent::Changed(answer) => changed(state, answer),
        PluginsIntent::Checked {
            reports,
            plugins,
            asked,
        } => {
            let request = if asked {
                Request::Check
            } else {
                Request::DailyCheck
            };
            let told: Vec<PluginsOutcome> = reports
                .iter()
                .filter_map(|report| note_report(state, report, request))
                .collect();

            let mut outcomes = take_plugins(state, plugins);
            outcomes.extend(told);
            outcomes
        },
        PluginsIntent::Reloaded {
            plugin,
            outcome,
            plugins,
        } => {
            let told = note_outcome(state, &plugin, outcome, Request::Reload);

            let mut outcomes = take_plugins(state, plugins);
            outcomes.extend(told);
            outcomes
        },
        PluginsIntent::UpdateTick => vec![PluginsOutcome::CheckUpdatesIfDue],
        PluginsIntent::Running(running) => {
            state.set_running(running);
            Vec::new()
        },
    }
}

/// Whether `intent` is one the items of the menu of a plugin send.
fn is_menu_item(intent: &PluginsIntent) -> bool {
    matches!(
        intent,
        PluginsIntent::Reload(_)
            | PluginsIntent::CheckForUpdates(_)
            | PluginsIntent::Review(_)
            | PluginsIntent::ShowLog(_)
            | PluginsIntent::AskRemove(_)
    )
}

/// Shows `place` in the mode: nothing selected for the list, the plugin
/// of "Read by" selected, the update dialog of Review, or Add plugin….
fn open(state: &mut PluginsState, place: PluginsPlace) -> Vec<PluginsOutcome> {
    state.close_dialog();

    match place {
        PluginsPlace::AddPlugin => {
            state.show(Dialog::AddUrl { url: String::new() });
            vec![PluginsOutcome::FocusField(URL_FIELD)]
        },
        PluginsPlace::Plugin(plugin) => {
            state.select(Some(plugin));
            Vec::new()
        },
        PluginsPlace::Review(plugin) => {
            if state.waiting_update(&plugin).is_some() {
                state.show(Dialog::Update {
                    plugin: plugin.clone(),
                });
            }
            state.select(Some(plugin));
            Vec::new()
        },
        _ => {
            state.select(None);
            Vec::new()
        },
    }
}

/// Next under the link or the folder: find the plugin there.
fn find(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }

    let outcome = match state.dialog() {
        Some(Dialog::AddUrl { url }) if !url.trim().is_empty() => {
            PluginsOutcome::FindRelease(url.trim().to_string())
        },
        Some(Dialog::AddFolder { folder }) if !folder.trim().is_empty() => {
            PluginsOutcome::FindLocal(typed_folder(folder.trim(), state.home()))
        },
        _ => return Vec::new(),
    };
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

fn install(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }
    let Some(Dialog::Install { plan }) = state.dialog() else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::Install { plan: plan.clone() };
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

fn set_part_on(
    state: &mut PluginsState,
    plugin: String,
    part: PartRef,
    on: bool,
) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }

    state.set_error(None);
    state.set_busy(true);

    vec![PluginsOutcome::SetPartOn { plugin, part, on }]
}

/// Turn on of the need "<Part> of the <Plugin> plugin is off".
/// Try again of a paused part: its count of failures starts from zero and
/// Agents mode reads again.
fn try_again(
    state: &mut PluginsState,
    plugin: &str,
    part: &PartRef,
) -> Vec<PluginsOutcome> {
    let health = state.running_plugin(plugin).and_then(|running| {
        running
            .parts
            .iter()
            .find(|(loaded, _)| loaded == part)
            .map(|(_, health)| health)
    });
    let Some(health) = health else {
        return Vec::new();
    };

    health.try_again();

    vec![PluginsOutcome::ReadAgain]
}

fn apply_update(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }
    let Some(Dialog::Update { plugin }) = state.dialog() else {
        return Vec::new();
    };
    let Some(update) = state.waiting_update(plugin) else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::ApplyUpdate {
        update: Box::new(update.clone()),
    };
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

fn remove(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }
    let Some(Dialog::Remove(plugin)) = state.dialog() else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::Remove(plugin.clone());
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

/// A change finished: the dialog closes and the plugins are as they are
/// now, a new one selected, and an update the user agreed to says so in a
/// notification; or the reason it failed, in the open dialog or on the
/// page.
fn changed(
    state: &mut PluginsState,
    answer: ChangeAnswer,
) -> Vec<PluginsOutcome> {
    state.set_busy(false);
    let plugins = match answer {
        Ok(plugins) => plugins,
        Err(error) => {
            state.set_error(Some(error_text(&error)));
            return Vec::new();
        },
    };

    let mut told = None;
    let installed = match state.dialog().cloned() {
        Some(Dialog::Update { plugin }) => {
            told = state
                .waiting_update(&plugin)
                .map(|update| update.to().to_string())
                .map(|version| updated(state, &plugin, &version));
            state.forget_news(&plugin);
            None
        },
        Some(Dialog::Remove(plugin)) => {
            state.forget_news(&plugin);
            None
        },
        Some(Dialog::Install { plan }) => {
            Some(plan.manifest().plugin().id().to_string())
        },
        _ => None,
    };
    state.close_dialog();

    let mut outcomes = take_plugins(state, plugins);
    if installed.is_some() {
        state.select(installed);
    }
    outcomes.extend(told);

    outcomes
}

/// Keeps `plugins`: the parts to load when they changed, then the notes
/// Agents mode shows of the plugins now.
fn take_plugins(
    state: &mut PluginsState,
    plugins: Vec<ListedPlugin>,
) -> Vec<PluginsOutcome> {
    let mut outcomes = Vec::new();

    if state.plugins() != plugins.as_slice() {
        let parts = plugins.iter().map(ListedPlugin::parts_to_load).collect();
        state.set_plugins(plugins);
        outcomes.push(PluginsOutcome::LoadParts(parts));
    }
    outcomes.push(PluginsOutcome::Notes(notes(state)));

    outcomes
}

/// What `report` says of its plugin: on its page a newer version that
/// needs a newer OTTY, and the notification of the check, if any.
fn note_report(
    state: &mut PluginsState,
    report: &UpdateReport,
    request: Request,
) -> Option<PluginsOutcome> {
    let plugin = report.plugin_id();
    let too_new = report
        .too_new()
        .map(|skipped| skipped.tag().trim_start_matches('v').to_string());

    state.set_too_new(plugin, too_new);
    note_outcome(state, plugin, report.outcome().clone(), request)
}

/// What `outcome` of `request` says of the plugin `plugin`: an update that
/// waits stays on its page, a failure signs its row failed; the rest is
/// the notification returned. A reload that kept the version says
/// "reloaded", a new version "updated to", and Check for updates with
/// nothing newer "up to date".
fn note_outcome(
    state: &mut PluginsState,
    plugin: &str,
    outcome: Result<UpdateOutcome, InstallError>,
    request: Request,
) -> Option<PluginsOutcome> {
    let name = plugin_name(state, plugin);

    match outcome {
        Ok(UpdateOutcome::Waiting(update)) => {
            state.set_news(plugin, PluginNews::Waiting(update));
            None
        },
        Ok(UpdateOutcome::Updated { from, to })
            if request != Request::Reload || from != to =>
        {
            state.forget_news(plugin);
            Some(updated(state, plugin, &to.to_string()))
        },
        Err(error) => {
            let reason = error_text(&error);
            let key = match request {
                Request::Reload => Key::TplPluginsReloadFailed,
                _ => Key::TplPluginsCheckFailed,
            };
            state.set_news(plugin, failed(&error));
            Some(PluginsOutcome::Failed(i18n::filled(
                key,
                &[("name", &name), ("reason", &reason)],
            )))
        },
        Ok(_) => {
            if matches!(state.news_of(plugin), Some(PluginNews::Failed(_))) {
                state.forget_news(plugin);
            }
            let key = match request {
                Request::Reload => Key::TplPluginsReloaded,
                Request::Check => Key::TplPluginsUpToDate,
                Request::DailyCheck => return None,
            };
            Some(PluginsOutcome::Done(i18n::filled(key, &[("name", &name)])))
        },
    }
}

/// "Plugin Example updated to 0.5.0".
fn updated(
    state: &PluginsState,
    plugin: &str,
    version: &str,
) -> PluginsOutcome {
    let name = plugin_name(state, plugin);

    PluginsOutcome::Done(i18n::filled(
        Key::TplPluginsUpdated,
        &[("name", &name), ("version", version)],
    ))
}

/// The name of the installed plugin `plugin`, or its id.
fn plugin_name(state: &PluginsState, plugin: &str) -> String {
    state
        .plugin(plugin)
        .map(|listed| listed.installed.name().to_string())
        .unwrap_or_else(|| plugin.to_string())
}

/// The repository of the plugin `plugin`.
fn repository_of(state: &PluginsState, plugin: &str) -> Option<String> {
    state.plugin(plugin)?.repository().map(String::from)
}

/// A folder as the user typed it, `~` standing for `home`.
fn typed_folder(typed: &str, home: &Path) -> PathBuf {
    if typed == "~" {
        return home.to_path_buf();
    }

    match typed.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(typed),
    }
}

#[cfg(test)]
mod tests;
