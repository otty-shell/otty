//! Delivers the intents of Plugins mode and reads what it reports: the
//! installs, updates and removals `otty-app-plugins-io` runs, the parts
//! Agents mode loads when the installed plugins change, and what Agents
//! mode shows of them. Settings and Agents mode open the mode at a place
//! through [`open_plugins_mode`].

use std::sync::Arc;

use iced::Task;
use iced::widget::operation;
use otty_app_agents::AgentsIntent;
use otty_app_agents_io::{self as agents_io, LoadedReaders, RunningParts};
use otty_app_domain::mode::Mode;
use otty_app_domain::plugins::PluginsPlace;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_plugins::{
    Plugins, PluginsCtx, PluginsIntent, PluginsOutcome, RunningPlugin,
};
use otty_app_plugins_io as plugins_io;

use super::{Delivery, deliver};
use crate::app::{App, hidden_entries, installed_parts, read_variable};
use crate::message::AppEvent;

/// Plugins mode gets the pointer and the size of the main area for the
/// menu of a plugin, and how many of OTTY's own logins each plugin made,
/// which its Remove dialog names, and reads what it reports.
impl Delivery for PluginsIntent {
    type Component = Plugins;

    fn parts(app: &mut App) -> (&mut Plugins, PluginsCtx) {
        let ctx = PluginsCtx {
            cursor: app.sidebar.cursor(),
            screen_size: app.screen_size,
            own_logins: app.agents.own_logins_by_plugin(),
        };

        (&mut app.plugins, ctx)
    }

    fn read(app: &mut App, outcome: PluginsOutcome) -> Task<AppEvent> {
        let installer = &app.plugin_io.installer;

        match outcome {
            PluginsOutcome::FocusField(id) => operation::focus(id),
            PluginsOutcome::FindRelease(url) => {
                plugins_io::find_release(installer, url).map(AppEvent::Plugins)
            },
            PluginsOutcome::FindLocal(folder) => {
                plugins_io::find_local(installer, folder).map(AppEvent::Plugins)
            },
            PluginsOutcome::Install { plan } => {
                plugins_io::install(installer, *plan).map(AppEvent::Plugins)
            },
            PluginsOutcome::SetPartOn { plugin, part, on } => {
                plugins_io::set_part_on(installer, plugin, part, on)
                    .map(AppEvent::Plugins)
            },
            PluginsOutcome::Remove(plugin) => {
                plugins_io::remove(installer, plugin).map(AppEvent::Plugins)
            },
            // The plugin's folder held its own logins: Agents mode forgets
            // them in `agents.json` (issue 13 of spec
            // 2026-11-23-accounts-apart-from-profiles).
            PluginsOutcome::Removed(plugin) => {
                deliver(app, AgentsIntent::PluginRemoved(plugin))
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
            PluginsOutcome::ApplyUpdate { update } => {
                plugins_io::apply_update(installer, *update)
                    .map(AppEvent::Plugins)
            },
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
            // What Reload, Check for updates or an update did (issue 35).
            PluginsOutcome::Done(text) => {
                deliver(app, NotificationsIntent::PushDone(text))
            },
            PluginsOutcome::Failed(text) => {
                deliver(app, NotificationsIntent::Push(text))
            },
        }
    }
}

/// Show Plugins mode at `place`: Manage plugins…, "Read by",
/// Add plugin… and Review of Agents mode (issue 30).
pub(crate) fn open_plugins_mode(
    app: &mut App,
    place: PluginsPlace,
) -> Task<AppEvent> {
    let shown = deliver(app, ModesIntent::Select(Mode::Plugins));
    let opened = deliver(app, PluginsIntent::Open(place));

    Task::batch([shown, opened])
}

/// The parts of the installed plugins were loaded: Agents mode reads with
/// them and their provider parts from now on, and Plugins mode shows their
/// health and logs.
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

/// The loaded plugins as Plugins mode holds them.
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
    use iced_test::simulator;

    use super::*;
    use crate::testing::window::app;

    /// Issue 30: "Read by" of Agents mode shows Plugins mode; Esc in a
    /// dialog of the mode closes the dialog and stays in the mode.
    #[test]
    fn agents_mode_opens_plugins_mode_and_esc_closes_its_dialog() {
        use crate::guards::{Overlay, top_overlay};
        use crate::testing::keyboard::escape_press;
        use crate::update::update_input;

        let mut app = app();

        let _ = deliver(
            &mut app,
            AgentsIntent::OpenPlugin(String::from("example")),
        );
        let in_mode = app.modes.active();
        let _ =
            update_input(&mut app, AppEvent::Plugins(PluginsIntent::AddPlugin));
        let on_top = top_overlay(&app);
        let _ = update_input(&mut app, AppEvent::Keyboard(escape_press()));

        assert_eq!(in_mode, Mode::Plugins);
        assert_eq!(on_top, Some(Overlay::PluginsDialog));
        assert!(!app.plugins.has_dialog());
        assert_eq!(app.modes.active(), Mode::Plugins);
    }

    /// Issue 35: what Reload or Check for updates did, or why it failed,
    /// is a toast in the window.
    #[test]
    fn the_results_of_plugins_mode_are_notifications() {
        let mut app = app();

        let _done = PluginsIntent::read(
            &mut app,
            PluginsOutcome::Done(String::from("Plugin Example reloaded")),
        );
        let _failed = PluginsIntent::read(
            &mut app,
            PluginsOutcome::Failed(String::from(
                "Plugin Example could not be reloaded: no file",
            )),
        );

        let mut ui = simulator(crate::view::view(&app));
        assert!(ui.find("Plugin Example reloaded").is_ok());
        assert!(
            ui.find("Plugin Example could not be reloaded: no file")
                .is_ok()
        );
        assert_eq!(app.notifications.unseen_count(), 2);
    }

    /// Issue 13 of spec 2026-11-23-accounts-apart-from-profiles: Plugins
    /// mode is told how many own logins each plugin made, and once a
    /// plugin is removed its own logins leave Agents mode and
    /// `agents.json`.
    #[test]
    fn a_removed_plugin_takes_its_own_logins_out_of_agents_mode() {
        use std::collections::BTreeMap;

        use otty_agents::{AgentId, OwnLogin, ProviderId, SavedProfiles};

        use crate::testing::agents::saved_loaded;

        let login = |id: &str, plugin: &str| {
            OwnLogin::new(
                id,
                plugin,
                ProviderId::new(plugin),
                AgentId::new("coder"),
            )
        };
        let saved = SavedProfiles::default()
            .with_found_report_shown()
            .with_own_login(login("a1", "openai"))
            .with_own_login(login("b2", "anthropic"));
        let mut app = app();
        let _ = deliver(&mut app, saved_loaded(saved));
        let (_, asked) = PluginsIntent::parts(&mut app);

        let _ = PluginsIntent::read(
            &mut app,
            PluginsOutcome::Removed(String::from("openai")),
        );

        assert_eq!(
            asked.own_logins,
            BTreeMap::from([
                (String::from("anthropic"), 1),
                (String::from("openai"), 1),
            ])
        );
        assert_eq!(
            app.agents.own_logins_by_plugin(),
            BTreeMap::from([(String::from("anthropic"), 1)])
        );
    }

    /// A dialog of Plugins mode is not on screen in another mode, so it
    /// takes no input there.
    #[test]
    fn the_dialog_of_plugins_mode_counts_only_in_the_mode() {
        use crate::guards::top_overlay;

        let mut app = app();
        let _ = open_plugins_mode(&mut app, PluginsPlace::AddPlugin);
        let in_mode = top_overlay(&app);

        let _ = deliver(&mut app, ModesIntent::Select(Mode::Terminal));

        assert!(in_mode.is_some());
        assert_eq!(top_overlay(&app), None);
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
