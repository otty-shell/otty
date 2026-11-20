//! What the Plugins page does with each intent: which dialog it shows,
//! what it asks `otty-app-plugins-io` to do, and what it tells the binary
//! when the installed plugins change.

use std::path::{Path, PathBuf};

use otty_app_domain::plugins::{OffPart, PartSide, PluginsPlace};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{InstallError, UpdateOutcome, UpdateReport};

use crate::event::{ChangeAnswer, PluginsIntent, PluginsOutcome};
use crate::model::{failed, given_to_new};
use crate::notes::notes;
use crate::state::{PluginNews, PluginsState, Screen};
use crate::types::ListedPlugin;
use crate::view::{FOLDER_FIELD, URL_FIELD};
use crate::words::error_text;

/// Reduce `intent` into state changes and outcomes.
pub(crate) fn reduce(
    state: &mut PluginsState,
    intent: PluginsIntent,
) -> Vec<PluginsOutcome> {
    match intent {
        PluginsIntent::Open(place) => open(state, place),
        PluginsIntent::Close => close(state),
        PluginsIntent::Escape => {
            if *state.screen() == Screen::List {
                close(state)
            } else {
                state.show(Screen::List);
                Vec::new()
            }
        },
        PluginsIntent::Back => {
            state.show(Screen::List);
            Vec::new()
        },
        PluginsIntent::AddPlugin => {
            state.show(Screen::AddUrl { url: String::new() });
            vec![PluginsOutcome::FocusField(URL_FIELD)]
        },
        PluginsIntent::AddFromFolder => {
            state.show(Screen::AddFolder {
                folder: String::new(),
            });
            vec![PluginsOutcome::FocusField(FOLDER_FIELD)]
        },
        PluginsIntent::UrlChanged(text) => {
            if let Screen::AddUrl { url } = state.screen_mut() {
                *url = text;
            }
            Vec::new()
        },
        PluginsIntent::FolderChanged(text) => {
            if let Screen::AddFolder { folder } = state.screen_mut() {
                *folder = text;
            }
            Vec::new()
        },
        PluginsIntent::Find => find(state),
        PluginsIntent::Found(found) => {
            state.set_busy(false);
            match found {
                Ok(plan) => state.show(Screen::Install {
                    plan,
                    use_new: Vec::new(),
                }),
                Err(error) => state.set_error(Some(error_text(&error))),
            }
            Vec::new()
        },
        PluginsIntent::UseNew(part, use_new) => {
            choose(state, part, use_new);
            Vec::new()
        },
        PluginsIntent::Install => install(state),
        PluginsIntent::SetPartOn { plugin, part, on } => {
            set_part_on(state, plugin, part, on)
        },
        PluginsIntent::TurnOn(off) => turn_on(state, &off),
        PluginsIntent::TryAgain { plugin, part } => {
            try_again(state, &plugin, &part)
        },
        PluginsIntent::ToggleActions(plugin) => {
            let is_open = state.actions_open() == Some(plugin.as_str());
            state.set_actions_open((!is_open).then_some(plugin));
            Vec::new()
        },
        PluginsIntent::CheckForUpdates(plugin) => {
            state.set_actions_open(None);
            vec![PluginsOutcome::CheckUpdate(plugin)]
        },
        PluginsIntent::Reload(plugin) => {
            state.set_actions_open(None);
            vec![PluginsOutcome::Reload(plugin)]
        },
        PluginsIntent::Review(plugin) => {
            open(state, PluginsPlace::Review(plugin))
        },
        PluginsIntent::ApplyUpdate => apply_update(state),
        PluginsIntent::ShowPermissions(plugin) => {
            state.show(Screen::Permissions(plugin));
            Vec::new()
        },
        PluginsIntent::ShowLog(plugin) => {
            state.show(Screen::Log(plugin));
            Vec::new()
        },
        PluginsIntent::OpenRepository(plugin) => {
            state.set_actions_open(None);
            repository_of(state, &plugin)
                .map(PluginsOutcome::OpenPage)
                .into_iter()
                .collect()
        },
        PluginsIntent::AskRemove(plugin) => {
            state.show(Screen::Remove(plugin));
            Vec::new()
        },
        PluginsIntent::Remove => remove(state),
        PluginsIntent::Changed(answer) => changed(state, answer),
        PluginsIntent::Checked {
            reports,
            plugins,
            asked,
        } => {
            for report in &reports {
                note_report(state, report, asked);
            }
            take_plugins(state, plugins)
        },
        PluginsIntent::Reloaded {
            plugin,
            outcome,
            plugins,
        } => {
            note_outcome(state, &plugin, outcome, true);
            take_plugins(state, plugins)
        },
        PluginsIntent::UpdateTick => vec![PluginsOutcome::CheckUpdatesIfDue],
        PluginsIntent::Running(running) => {
            state.set_running(running);
            Vec::new()
        },
    }
}

/// Opens the page at `place`.
fn open(state: &mut PluginsState, place: PluginsPlace) -> Vec<PluginsOutcome> {
    let mut outcomes = if state.is_open() {
        Vec::new()
    } else {
        vec![PluginsOutcome::Opened]
    };
    state.set_open(true);
    state.set_highlighted(None);

    match place {
        PluginsPlace::AddPlugin => {
            state.show(Screen::AddUrl { url: String::new() });
            outcomes.push(PluginsOutcome::FocusField(URL_FIELD));
        },
        PluginsPlace::Plugin(plugin) => {
            state.show(Screen::List);
            state.set_highlighted(Some(plugin));
        },
        PluginsPlace::Review(plugin) => {
            if state.waiting_update(&plugin).is_some() {
                state.show(Screen::Update {
                    plugin,
                    use_new: Vec::new(),
                });
            } else {
                state.show(Screen::List);
                state.set_highlighted(Some(plugin));
            }
        },
        _ => state.show(Screen::List),
    }

    outcomes
}

fn close(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if !state.is_open() {
        return Vec::new();
    }

    state.set_open(false);
    state.show(Screen::List);
    state.set_highlighted(None);

    vec![PluginsOutcome::Closed]
}

/// Next under the link or the folder: find the plugin there.
fn find(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }

    let outcome = match state.screen() {
        Screen::AddUrl { url } if !url.trim().is_empty() => {
            PluginsOutcome::FindRelease(url.trim().to_string())
        },
        Screen::AddFolder { folder } if !folder.trim().is_empty() => {
            PluginsOutcome::FindLocal(typed_folder(folder.trim(), state.home()))
        },
        _ => return Vec::new(),
    };
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

/// Gives what another plugin serves to the new `part`, or keeps it with
/// the plugin that serves it now.
fn choose(state: &mut PluginsState, part: PartRef, give: bool) {
    let use_new = match state.screen_mut() {
        Screen::Install { use_new, .. } | Screen::Update { use_new, .. } => {
            use_new
        },
        _ => return,
    };

    use_new.retain(|chosen| *chosen != part);
    if give {
        use_new.push(part);
    }
}

fn install(state: &mut PluginsState) -> Vec<PluginsOutcome> {
    if state.is_busy() {
        return Vec::new();
    }
    let Screen::Install { plan, use_new } = state.screen() else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::Install {
        use_new_for: given_to_new(plan.conflicts(), use_new),
        plan: plan.clone(),
    };
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
fn turn_on(state: &mut PluginsState, off: &OffPart) -> Vec<PluginsOutcome> {
    let part = match off.side {
        PartSide::Agent => PartRef::agent(&off.part),
        PartSide::Provider => PartRef::provider(&off.part),
    };

    set_part_on(state, off.plugin.clone(), part, true)
}

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
    let Screen::Update { plugin, use_new } = state.screen() else {
        return Vec::new();
    };
    let Some(update) = state.waiting_update(plugin) else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::ApplyUpdate {
        use_new_for: given_to_new(update.conflicts(), use_new),
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
    let Screen::Remove(plugin) = state.screen() else {
        return Vec::new();
    };

    let outcome = PluginsOutcome::Remove(plugin.clone());
    state.set_error(None);
    state.set_busy(true);

    vec![outcome]
}

/// A change finished: back to the list with the plugins as they are now,
/// or the reason it failed in the open dialog.
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

    match state.screen().clone() {
        Screen::Update { plugin, .. } => {
            let version = state
                .waiting_update(&plugin)
                .map(|update| update.to().to_string());
            match version {
                Some(version) => {
                    state.set_news(&plugin, PluginNews::Updated(version));
                },
                None => state.forget_news(&plugin),
            }
            state.show(Screen::List);
        },
        Screen::Remove(plugin) => {
            state.forget_news(&plugin);
            state.show(Screen::List);
        },
        Screen::Install { .. } => state.show(Screen::List),
        _ => state.set_error(None),
    }

    take_plugins(state, plugins)
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

/// What `report` says of its plugin, on its row. `asked`: the user asked
/// for the check, so "Up to date" shows too.
fn note_report(state: &mut PluginsState, report: &UpdateReport, asked: bool) {
    let plugin = report.plugin_id();
    let too_new = report
        .too_new()
        .map(|skipped| skipped.tag().trim_start_matches('v').to_string());

    state.set_too_new(plugin, too_new);
    note_outcome(state, plugin, report.outcome().clone(), asked);
}

/// What `outcome` says of the plugin `plugin`, on its row.
fn note_outcome(
    state: &mut PluginsState,
    plugin: &str,
    outcome: Result<UpdateOutcome, InstallError>,
    asked: bool,
) {
    match outcome {
        Ok(UpdateOutcome::Updated { to, .. }) => {
            state.set_news(plugin, PluginNews::Updated(to.to_string()));
        },
        Ok(UpdateOutcome::Waiting(update)) => {
            state.set_news(plugin, PluginNews::Waiting(update));
        },
        Ok(UpdateOutcome::UpToDate) if asked => {
            state.set_news(plugin, PluginNews::UpToDate);
        },
        Ok(_) => {
            if matches!(state.news_of(plugin), Some(PluginNews::Failed(_))) {
                state.forget_news(plugin);
            }
        },
        Err(error) => state.set_news(plugin, failed(&error)),
    }
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
