//! Delivers the intents of the Plugins page and reads what it reports: the
//! installs, updates and removals `otty-app-plugins-io` runs, the parts
//! Agents mode loads when the installed plugins change, and what Agents
//! mode shows of them.

use std::sync::Arc;

use iced::Task;
use iced::widget::operation;
use otty_app_agents::AgentsIntent;
use otty_app_agents_io::{self as agents_io, LoadedReaders, RunningParts};
use otty_app_plugins::{Plugins, PluginsIntent, PluginsOutcome, RunningPlugin};
use otty_app_plugins_io as plugins_io;
use otty_app_terminal::TerminalIntent;

use super::{Delivery, deliver};
use crate::app::{App, hidden_entries, installed_parts, read_variable};
use crate::message::AppEvent;

/// The Plugins page reads what it reports.
impl Delivery for PluginsIntent {
    type Component = Plugins;

    fn parts(app: &mut App) -> (&mut Plugins, ()) {
        (&mut app.plugins, ())
    }

    fn read(app: &mut App, outcome: PluginsOutcome) -> Task<AppEvent> {
        let installer = &app.plugin_io.installer;

        match outcome {
            // The open page takes the keyboard from the terminal, so typed
            // keys do not reach the shell and Esc reaches the frame.
            PluginsOutcome::Opened => super::take_keyboard(),
            PluginsOutcome::Closed => deliver(app, TerminalIntent::FocusActive),
            PluginsOutcome::FocusField(id) => operation::focus(id),
            PluginsOutcome::FindRelease(url) => {
                plugins_io::find_release(installer, url).map(AppEvent::Plugins)
            },
            PluginsOutcome::FindLocal(folder) => {
                plugins_io::find_local(installer, folder).map(AppEvent::Plugins)
            },
            PluginsOutcome::Install { plan, use_new_for } => {
                plugins_io::install(installer, *plan, use_new_for)
                    .map(AppEvent::Plugins)
            },
            PluginsOutcome::SetPartOn { plugin, part, on } => {
                plugins_io::set_part_on(installer, plugin, part, on)
                    .map(AppEvent::Plugins)
            },
            PluginsOutcome::Remove(plugin) => {
                plugins_io::remove(installer, plugin).map(AppEvent::Plugins)
            },
            PluginsOutcome::CheckUpdate(plugin) => {
                plugins_io::check_update(installer, plugin)
                    .map(AppEvent::Plugins)
            },
            PluginsOutcome::CheckUpdatesIfDue => {
                plugins_io::check_updates_if_due(installer)
                    .map(AppEvent::Plugins)
            },
            PluginsOutcome::Reload(plugin) => {
                plugins_io::reload(installer, plugin).map(AppEvent::Plugins)
            },
            PluginsOutcome::ApplyUpdate {
                update,
                use_new_for,
            } => plugins_io::apply_update(installer, *update, use_new_for)
                .map(AppEvent::Plugins),
            PluginsOutcome::OpenPage(address) => {
                agents_io::open_page(address).map(AppEvent::Agents)
            },
            PluginsOutcome::CopyText(text) => {
                agents_io::copy_text(text).map(AppEvent::Agents)
            },
            PluginsOutcome::LoadParts(parts) => agents_io::load_readers(
                Arc::clone(&app.plugin_io.engine),
                installed_parts(parts),
                app.agents.env().home().to_path_buf(),
                read_variable,
                hidden_entries(),
            )
            .map(AppEvent::PluginsLoaded),
            PluginsOutcome::Notes(notes) => {
                deliver(app, AgentsIntent::PluginNotesChanged(notes))
            },
            // Try again started a paused part: Agents mode reads again.
            PluginsOutcome::ReadAgain => deliver(app, AgentsIntent::Refresh),
        }
    }
}

/// The parts of the installed plugins were loaded: Agents mode reads with
/// them and their provider parts from now on, and the Plugins page shows
/// their health and logs.
pub(crate) fn loaded_plugins(
    app: &mut App,
    loaded: LoadedReaders,
) -> Task<AppEvent> {
    app.plugin_providers = loaded.providers;
    app.language_receivers = loaded.language_receivers;
    let running = running_plugins(loaded.running);

    let agents = deliver(app, AgentsIntent::ReadersChanged(loaded.readers));
    let plugins = deliver(app, PluginsIntent::Running(running));

    Task::batch([agents, plugins])
}

/// The loaded plugins as the Plugins page holds them.
pub(crate) fn running_plugins(
    running: Vec<RunningParts>,
) -> Vec<RunningPlugin> {
    running
        .into_iter()
        .map(|running| RunningPlugin {
            plugin: running.plugin,
            log: running.log,
            parts: running.parts,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_app_domain::plugins::PluginsPlace;

    use super::*;
    use crate::testing::task_outputs::focuses_nothing;
    use crate::testing::window::app;

    /// The open page takes the keyboard from the terminal, as any
    /// overlay does.
    #[test]
    fn the_open_page_takes_the_keyboard() {
        let mut app = app();

        let task = deliver(&mut app, PluginsIntent::Open(PluginsPlace::List));

        assert!(app.plugins.is_open());
        assert!(focuses_nothing(task));
    }

    /// Spec Decision 12: the page opens from Settings, over it; Esc goes
    /// back from a dialog of the page, then closes it, and Settings stays
    /// with its edits.
    #[test]
    fn the_page_opens_over_settings_and_esc_returns_to_settings() {
        use otty_app_settings::SettingsIntent;

        use crate::guards::{Overlay, top_overlay};
        use crate::testing::keyboard::escape_press;
        use crate::testing::window::{edit_the_shell, open_settings};
        use crate::update::update_input;

        let mut app = app();
        open_settings(&mut app);
        edit_the_shell(&mut app, "/bin/edited");

        let _ = update_input(
            &mut app,
            AppEvent::Settings(SettingsIntent::OpenPlugins),
        );
        let on_top = top_overlay(&app);
        let _ =
            update_input(&mut app, AppEvent::Plugins(PluginsIntent::AddPlugin));
        let _ = update_input(&mut app, AppEvent::Keyboard(escape_press()));
        let still_open = app.plugins.is_open();
        let _ = update_input(&mut app, AppEvent::Keyboard(escape_press()));

        assert_eq!(on_top, Some(Overlay::Plugins));
        assert!(still_open, "Esc in Add plugin goes back to the list");
        assert!(!app.plugins.is_open());
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
        assert!(app.settings.has_unsaved_edits());
    }

    /// "Read by" and Add plugin… of Agents mode open the page.
    #[test]
    fn agents_mode_opens_the_page_at_a_plugin() {
        let mut app = app();

        let _ = deliver(
            &mut app,
            AgentsIntent::OpenPlugin(String::from("example")),
        );

        assert!(app.plugins.is_open());
    }

    /// Issue 07, left for issue 09: the loaded parts give Agents mode its
    /// readers and the provider parts that read the accounts.
    #[test]
    fn loaded_plugins_reach_agents_mode() {
        let mut app = app();
        let loaded = agents_io::readers_now(
            &app.plugin_io.engine,
            &[],
            std::env::temp_dir(),
            |_| None,
            &[String::from("codex")],
        );

        let _none = loaded_plugins(&mut app, loaded);

        assert!(
            app.agents.known_agents().iter().all(|agent| agent
                .entry()
                .id()
                .as_str()
                != "codex")
        );
        assert!(app.plugin_providers.is_empty());
    }
}
