//! The work the Plugins page asks for, each on a thread of its own: it may
//! reach GitHub, copy and check files and compile a plugin, so it never
//! holds up the window. Each answers with the page's intent.

use std::path::PathBuf;
use std::time::SystemTime;

use iced::Task;
use iced::futures::channel::oneshot;
use otty_app_plugins::{ChangeAnswer, ListedPlugin, PluginsIntent};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{
    InstallError, InstallPlan, Installer, PendingUpdate, Served, UpdateOutcome,
    UpdateReport, embedded_plugins,
};

use crate::installed::{SharedInstaller, listed};

/// Finds the newest release of the GitHub repository at `url` this OTTY
/// can run; answers [`PluginsIntent::Found`].
pub fn find_release(
    shared: &SharedInstaller,
    url: String,
) -> Task<PluginsIntent> {
    let shared = shared.clone();

    on_own_thread(
        move || {
            shared
                .with(|installer| installer.find_release(&url, false))
                .map(Box::new)
        },
        stopped,
        PluginsIntent::Found,
    )
}

/// Finds the plugin in the local folder `folder`; answers
/// [`PluginsIntent::Found`].
pub fn find_local(
    shared: &SharedInstaller,
    folder: PathBuf,
) -> Task<PluginsIntent> {
    let shared = shared.clone();

    on_own_thread(
        move || {
            shared
                .with(|installer| installer.find_local(&folder))
                .map(Box::new)
        },
        stopped,
        PluginsIntent::Found,
    )
}

/// Installs the plugin of `plan`, giving the new plugin each id and host
/// of `use_new_for`; answers [`PluginsIntent::Changed`].
pub fn install(
    shared: &SharedInstaller,
    plan: InstallPlan,
    use_new_for: Vec<Served>,
) -> Task<PluginsIntent> {
    change(shared, move |installer| {
        installer.install(&plan, &use_new_for)
    })
}

/// Turns the part `part` of the plugin `plugin` on or off; answers
/// [`PluginsIntent::Changed`].
pub fn set_part_on(
    shared: &SharedInstaller,
    plugin: String,
    part: PartRef,
    on: bool,
) -> Task<PluginsIntent> {
    change(shared, move |installer| {
        installer.set_part_on(&plugin, &part, on)
    })
}

/// Removes the plugin `plugin`; answers [`PluginsIntent::Changed`].
pub fn remove(shared: &SharedInstaller, plugin: String) -> Task<PluginsIntent> {
    change(shared, move |installer| installer.remove(&plugin))
}

/// Installs the waiting `update` the user agreed to, giving it each id
/// and host of `use_new_for`; answers [`PluginsIntent::Changed`].
pub fn apply_update(
    shared: &SharedInstaller,
    update: PendingUpdate,
    use_new_for: Vec<Served>,
) -> Task<PluginsIntent> {
    change(shared, move |installer| {
        installer.apply_update(&update, &use_new_for)
    })
}

/// Check for updates of the plugin `plugin`; answers
/// [`PluginsIntent::Checked`].
pub fn check_update(
    shared: &SharedInstaller,
    plugin: String,
) -> Task<PluginsIntent> {
    checks(shared, true, move |installer| {
        vec![installer.check_update(&plugin)]
    })
}

/// The daily update check of every plugin installed from releases, when a
/// day has passed since the last one; answers [`PluginsIntent::Checked`],
/// with no reports when it is not due.
pub fn check_updates_if_due(shared: &SharedInstaller) -> Task<PluginsIntent> {
    checks(shared, false, |installer| {
        let now = SystemTime::now();
        if !installer.is_update_check_due(now) {
            return Vec::new();
        }

        installer.check_updates(now)
    })
}

/// Installs the plugins built into OTTY, as on every start; answers
/// [`PluginsIntent::Checked`].
pub fn install_built_in(shared: &SharedInstaller) -> Task<PluginsIntent> {
    checks(shared, false, |installer| {
        installer.install_embedded(&embedded_plugins())
    })
}

/// Reads the folder of the local plugin `plugin` again; answers
/// [`PluginsIntent::Reloaded`].
pub fn reload(shared: &SharedInstaller, plugin: String) -> Task<PluginsIntent> {
    let shared = shared.clone();
    let answer_plugin = plugin.clone();

    on_own_thread(
        move || {
            let outcome =
                shared.with(|installer| installer.reload_local(&plugin));
            (outcome, shared.plugins())
        },
        move || (stopped(), Vec::new()),
        move |(outcome, plugins): (
            Result<UpdateOutcome, InstallError>,
            Vec<ListedPlugin>,
        )| PluginsIntent::Reloaded {
            plugin: answer_plugin,
            outcome,
            plugins,
        },
    )
}

/// Runs the change `work` and answers with the installed plugins after it.
fn change(
    shared: &SharedInstaller,
    work: impl FnOnce(&mut Installer) -> Result<(), InstallError> + Send + 'static,
) -> Task<PluginsIntent> {
    let shared = shared.clone();

    on_own_thread(
        move || -> ChangeAnswer {
            shared.with(|installer| {
                work(installer)?;
                Ok(listed(installer))
            })
        },
        stopped,
        PluginsIntent::Changed,
    )
}

/// Runs the checks `work` and answers with their reports and the
/// installed plugins after them; `asked` when the user asked for them.
fn checks(
    shared: &SharedInstaller,
    asked: bool,
    work: impl FnOnce(&mut Installer) -> Vec<UpdateReport> + Send + 'static,
) -> Task<PluginsIntent> {
    let shared = shared.clone();

    on_own_thread(
        move || {
            let reports = shared
                .with(|installer| Ok(work(installer)))
                .unwrap_or_default();
            (reports, shared.plugins())
        },
        || (Vec::new(), Vec::new()),
        move |(reports, plugins)| PluginsIntent::Checked {
            reports,
            plugins,
            asked,
        },
    )
}

/// What a change that stopped before it answered gives.
fn stopped<T>() -> Result<T, InstallError> {
    Err(InstallError::Disk(String::from(
        "The work on the plugins stopped before it finished",
    )))
}

/// Runs `work` on a thread of its own and answers with `answer` of what it
/// gave, or of what `stopped` gives when the thread could not start or
/// ended without an answer. Not on the runtime's blocking pool: closing
/// the runtime waits for that pool, and a download may be slow.
fn on_own_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    stopped: impl FnOnce() -> T + Send + 'static,
    answer: impl FnOnce(T) -> PluginsIntent + Send + 'static,
) -> Task<PluginsIntent> {
    Task::perform(
        async move {
            let (sender, receiver) = oneshot::channel();
            let thread = std::thread::Builder::new()
                .name(String::from("otty-plugins-io"))
                .spawn(move || {
                    let _ = sender.send(work());
                });
            if let Err(error) = thread {
                log::warn!("no thread for the work on the plugins: {error}");
                return None;
            }

            receiver.await.ok()
        },
        move |given| {
            answer(given.unwrap_or_else(|| {
                log::warn!(
                    "the work on the plugins stopped before it answered"
                );
                stopped()
            }))
        },
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use iced::futures::StreamExt;
    use iced::futures::executor::block_on;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_app_testing::temp_dir::TempDir;
    use otty_plugin_host::PluginEngine;

    use super::*;
    use crate::installed::places;

    fn run(task: Task<PluginsIntent>) -> PluginsIntent {
        let stream = into_stream(task).expect("a task");
        let actions: Vec<Action<PluginsIntent>> = block_on(stream.collect());

        actions
            .into_iter()
            .find_map(|action| match action {
                Action::Output(intent) => Some(intent),
                _ => None,
            })
            .expect("an answer")
    }

    fn template_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../assets/plugins/example")
    }

    fn shared_in(folder: &TempDir) -> SharedInstaller {
        let engine =
            PluginEngine::new(folder.path().join("cache")).expect("the engine");

        SharedInstaller::open(
            places(&folder.path().join("config"), &folder.path().join("data")),
            Arc::new(engine),
            Vec::new(),
        )
    }

    /// Issue 09: Add plugin from folder finds the template plugin, the
    /// install keeps it marked "Local", turning a part off leaves the
    /// other on, and Remove takes it away.
    #[test]
    fn a_local_plugin_installs_turns_a_part_off_and_is_removed() {
        let folder = TempDir::new("plugins-io-local");
        let shared = shared_in(&folder);

        let PluginsIntent::Found(Ok(plan)) =
            run(find_local(&shared, template_folder()))
        else {
            panic!("the template is found");
        };
        let installed = run(install(&shared, *plan, Vec::new()));
        let off = run(set_part_on(
            &shared,
            String::from("example"),
            PartRef::agent("example-agent"),
            false,
        ));
        let removed = run(remove(&shared, String::from("example")));

        let PluginsIntent::Changed(Ok(plugins)) = installed else {
            panic!("installed: {installed:?}");
        };
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].installed.trust(),
            otty_plugin_releases::Trust::Local
        );
        assert!(plugins[0].manifest.is_some());
        assert!(plugins[0].icon.is_some());
        let PluginsIntent::Changed(Ok(plugins)) = off else {
            panic!("off: {off:?}");
        };
        let parts = plugins[0].installed.parts();
        assert!(!parts[0].is_on());
        assert!(parts[1].is_on());
        assert!(
            matches!(removed, PluginsIntent::Changed(Ok(plugins)) if plugins.is_empty())
        );
    }

    #[test]
    fn a_folder_with_no_plugin_says_why() {
        let folder = TempDir::new("plugins-io-empty");
        let shared = shared_in(&folder);

        let found = run(find_local(&shared, folder.path().join("nothing")));

        assert!(matches!(found, PluginsIntent::Found(Err(_))));
    }

    #[test]
    fn the_daily_check_with_no_plugins_reports_nothing() {
        let folder = TempDir::new("plugins-io-daily");
        let shared = shared_in(&folder);

        let checked = run(check_updates_if_due(&shared));

        assert!(matches!(
            checked,
            PluginsIntent::Checked { reports, plugins, asked: false }
                if reports.is_empty() && plugins.is_empty()
        ));
    }
}
