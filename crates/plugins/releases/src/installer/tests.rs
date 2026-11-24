use std::time::{Duration, SystemTime};

use otty_plugin_host::PluginEngine;
use otty_plugin_manifest::{PartRef, PluginVersion};

use super::*;
use crate::compiler::MockPluginCompiler;
use crate::diff::PartChange;
use crate::github::OFFICIAL_ORGANIZATION;
use crate::package::{MANIFEST_FILE, sha256_hex};
use crate::plan::{UpdateOutcome, UpdateReport};
use crate::plugins_file::{Source, Trust};
use crate::testing::{
    FakeGithub, FakeRelease, TempDir, agent_only_wasm, example_files,
    needs_agent_0_2, places_in, rebuilt_wasm, test_installer, write_folder,
};

const REPO: &str = "someone/otty-plugin-example";
const FORK: &str = "other/otty-plugin-example-fork";
/// What an own login folder holds in the tests.
const OWN_LOGIN_TEXT: &str = r#"{"tokens":"made-up"}"#;

fn url(repo: &str) -> String {
    format!("https://github.com/{repo}")
}

fn agent() -> PartRef {
    PartRef::agent("example-agent")
}

fn provider() -> PartRef {
    PartRef::provider("example-cloud")
}

/// Installs the newest release of `repo`.
fn install_from(installer: &mut Installer, repo: &str) {
    let plan = installer.find_release(&url(repo), false).expect("a plan");

    installer.install(&plan).expect("installed");
}

/// The template as another plugin, `example-fork`, with the same parts.
fn fork_release(tag: &str, version: &str) -> FakeRelease {
    FakeRelease::example(tag, version)
        .with_manifest_edit("id = \"example\"", "id = \"example-fork\"")
}

/// The template with its agent part only, whose `.wasm` exports the agent
/// contract only.
fn agent_only(release: FakeRelease) -> FakeRelease {
    let text = String::from_utf8(example_files("0.1.0")[MANIFEST_FILE].clone())
        .expect("text");
    let provider_half = text
        .split_once("[[agents.provider]]")
        .expect("a provider")
        .1;

    release
        .with_manifest_edit(provider_half, "")
        .with_manifest_edit("[[agents.provider]]", "")
        .with_file("example.wasm", &agent_only_wasm())
}

fn part_is_on(installer: &Installer, plugin: &str, part: &PartRef) -> bool {
    installer
        .plugin(plugin)
        .and_then(|plugin| plugin.part(part))
        .expect("the part")
        .is_on()
}

fn version_of(installer: &Installer, plugin: &str) -> String {
    installer
        .plugin(plugin)
        .expect("installed")
        .version()
        .to_string()
}

/// A file of OTTY's own login `6f0c1e2a` of the plugin `plugin` installed
/// under `folder`.
fn own_login_file(folder: &Path, plugin: &str) -> PathBuf {
    folder
        .join("data/plugins")
        .join(plugin)
        .join("accounts/6f0c1e2a/auth.json")
}

fn write_own_login(file: &Path) {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).expect("the own login folder");
    }
    std::fs::write(file, OWN_LOGIN_TEXT).expect("the own login");
}

#[test]
fn a_release_installs_into_its_version_folder_and_plugins_json() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);

    let plan = installer.find_release(&url(REPO), false).expect("a plan");
    installer.install(&plan).expect("installed");

    let plugin = installer.plugin("example").expect("installed");
    assert_eq!(plan.trust(), Trust::Community);
    assert_eq!(plugin.tag(), "v0.1.0");
    assert_eq!(*plugin.source(), Source::Release { url: url(REPO) });
    assert!(plugin.parts().iter().all(InstalledPart::is_on));
    assert!(
        plugin
            .part(&provider())
            .expect("provider")
            .granted()
            .contains(&"Sends requests to `api.example.com`".to_string())
    );
    let files = installer.folder_of(plugin);
    assert_eq!(files, folder.path().join("data/plugins/example/0.1.0"));
    let wasm = std::fs::read(files.join("example.wasm")).expect("the wasm");
    assert_eq!(plugin.wasm_sha256(), Some(sha256_hex(&wasm).as_str()));
    assert_eq!(plugin.hashes()["v0.1.0"].len(), 4);
    let reopened = test_installer(folder.path(), &github);
    assert_eq!(reopened.plugins(), installer.plugins());
}

#[test]
fn official_follows_the_one_constant() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let official = format!("{OFFICIAL_ORGANIZATION}/otty-plugin-example");
    github.publish(&official, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let installer = test_installer(folder.path(), &github);

    let plan = installer
        .find_release(&url(&official), false)
        .expect("a plan");

    assert_eq!(plan.trust(), Trust::Official);
}

#[test]
fn a_repository_with_no_semver_release_is_refused() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("latest", "0.1.0")]);
    let installer = test_installer(folder.path(), &github);

    let error = installer
        .find_release(&url(REPO), false)
        .expect_err("refused");

    assert_eq!(
        error.to_string(),
        "This repository has no release OTTY can install"
    );
}

#[test]
fn a_repository_whose_releases_all_need_a_newer_otty_names_the_contract() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(
        REPO,
        vec![needs_agent_0_2(FakeRelease::example("v0.2.0", "0.2.0"))],
    );
    let installer = test_installer(folder.path(), &github);

    let error = installer
        .find_release(&url(REPO), false)
        .expect_err("refused");

    assert_eq!(
        error.to_string(),
        "Needs a newer OTTY (agents.agent contract 0.2)"
    );
}

#[test]
fn a_plugin_id_taken_by_another_repository_is_refused_naming_it() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    github.publish(FORK, vec![FakeRelease::example("v0.3.0", "0.3.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);

    let taken = installer
        .find_release(&url(FORK), false)
        .expect_err("refused");
    let again = installer
        .find_release(&url(REPO), false)
        .expect_err("refused");

    assert_eq!(
        taken.to_string(),
        format!("A plugin `example` is already installed from {}", url(REPO))
    );
    assert_eq!(again, InstallError::AlreadyInstalled("example".to_string()));
}

/// Issue 29: a plugin whose parts serve the ids and hosts another plugin
/// serves installs with every part on, and the other plugin's parts stay
/// on: both read, and nothing is asked.
#[test]
fn a_plugin_that_serves_what_another_serves_installs_with_every_part_on() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    github.publish(FORK, vec![fork_release("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);

    install_from(&mut installer, FORK);

    for plugin in ["example", "example-fork"] {
        assert!(part_is_on(&installer, plugin, &agent()), "{plugin}");
        assert!(part_is_on(&installer, plugin, &provider()), "{plugin}");
    }
}

/// Turning a part on or off changes only that part.
#[test]
fn turning_a_part_on_leaves_the_parts_of_other_plugins_as_they_are() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    github.publish(FORK, vec![fork_release("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    install_from(&mut installer, FORK);

    installer
        .set_part_on("example-fork", &provider(), false)
        .expect("off");
    installer
        .set_part_on("example-fork", &provider(), true)
        .expect("on");

    assert!(part_is_on(&installer, "example-fork", &provider()));
    assert!(part_is_on(&installer, "example", &provider()));
}

#[test]
fn a_part_turned_off_stays_off_after_reopening() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);

    installer
        .set_part_on("example", &agent(), false)
        .expect("off");
    let missing =
        installer.set_part_on("example", &PartRef::agent("nope"), false);

    let reopened = test_installer(folder.path(), &github);
    assert!(!part_is_on(&reopened, "example", &agent()));
    assert!(matches!(missing, Err(InstallError::NoSuchPart { .. })));
}

#[test]
fn remove_deletes_the_folder_and_the_compiled_copy() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let wasm_hash = sha256_hex(&example_files("0.1.0")["example.wasm"]);
    let mut compiler = MockPluginCompiler::new();
    compiler.expect_compile().returning(|_, _| Ok(()));
    compiler
        .expect_remove_compiled()
        .withf(move |hash| hash == wasm_hash)
        .times(1)
        .returning(|_| Ok(()));
    let mut installer = Installer::open(
        places_in(folder.path()),
        Box::new(github.client()),
        Box::new(compiler),
    )
    .expect("an installer");
    let plan = installer.find_release(&url(REPO), false).expect("a plan");
    installer.install(&plan).expect("installed");

    installer.remove("example").expect("removed");

    assert!(installer.plugins().is_empty());
    assert!(!folder.path().join("data/plugins/example").exists());
    assert!(test_installer(folder.path(), &github).plugins().is_empty());
}

/// Issue 13 of spec 2026-11-23-accounts-apart-from-profiles: OTTY's own
/// logins of a plugin live in `<plugin>/accounts/`, so removing the plugin
/// removes them with it.
#[test]
fn remove_deletes_the_own_logins_of_the_plugin_too() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    let own_login = own_login_file(folder.path(), "example");
    write_own_login(&own_login);

    installer.remove("example").expect("removed");

    assert!(!own_login.exists());
    assert!(!folder.path().join("data/plugins/example/accounts").exists());
}

/// Issue 13: an update replaces only `<plugin>/<version>/`, so the own
/// logins in `<plugin>/accounts/` stay as they are, and `accounts` is
/// never taken for the old version folder.
#[test]
fn an_update_keeps_the_own_logins_of_the_plugin() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    let own_login = own_login_file(folder.path(), "example");
    write_own_login(&own_login);
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0"),
            FakeRelease::example("v0.2.0", "0.2.0")
                .with_file("example.wasm", &rebuilt_wasm()),
        ],
    );

    let report = installer.check_update("example");

    assert!(
        matches!(report.outcome(), Ok(UpdateOutcome::Updated { .. })),
        "{report:?}"
    );
    assert!(!folder.path().join("data/plugins/example/0.1.0").exists());
    assert_eq!(
        std::fs::read_to_string(&own_login).expect("the own login stays"),
        OWN_LOGIN_TEXT
    );
}

/// Issue 13: Reload of a local plugin replaces its version folder and
/// keeps its own logins.
#[test]
fn a_reload_keeps_the_own_logins_of_the_plugin() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let local = TempDir::new();
    write_folder(local.path(), &example_files("0.1.0"));
    let mut installer = test_installer(folder.path(), &github);
    let plan = installer.find_local(local.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    let own_login = own_login_file(folder.path(), "example");
    write_own_login(&own_login);
    std::fs::write(local.path().join("example.wasm"), rebuilt_wasm())
        .expect("rebuild");

    let reload = installer.reload_local("example");

    assert!(matches!(reload, Ok(UpdateOutcome::Updated { .. })));
    assert_eq!(
        std::fs::read_to_string(&own_login).expect("the own login stays"),
        OWN_LOGIN_TEXT
    );
}

/// A part the user turned off stays off when another plugin is removed.
#[test]
fn removing_a_plugin_leaves_a_part_turned_off_by_hand_off() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    github.publish(FORK, vec![fork_release("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    install_from(&mut installer, FORK);
    installer
        .set_part_on("example", &provider(), false)
        .expect("off");

    installer.remove("example-fork").expect("removed");

    assert!(!part_is_on(&installer, "example", &provider()));
    assert!(part_is_on(&installer, "example", &agent()));
}

/// Issue 29: an update that adds a part whose id another plugin serves
/// turns it on, and the other plugin's part stays on.
#[test]
fn an_update_that_adds_a_part_another_plugin_serves_turns_it_on() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(
        REPO,
        vec![agent_only(FakeRelease::example("v0.1.0", "0.1.0"))],
    );
    github.publish(FORK, vec![fork_release("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    install_from(&mut installer, FORK);
    github.publish(
        REPO,
        vec![
            agent_only(FakeRelease::example("v0.1.0", "0.1.0")),
            FakeRelease::example("v0.2.0", "0.2.0"),
        ],
    );
    let report = installer.check_update("example");
    let Ok(UpdateOutcome::Waiting(pending)) = report.outcome() else {
        panic!("waiting: {report:?}");
    };

    installer.apply_update(pending).expect("updated");

    assert_eq!(version_of(&installer, "example"), "0.2.0");
    assert!(part_is_on(&installer, "example", &provider()));
    assert!(part_is_on(&installer, "example-fork", &provider()));
}

#[test]
fn an_update_with_no_new_permission_installs_on_its_own() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let old_hash = sha256_hex(&example_files("0.1.0")["example.wasm"]);
    let mut compiler = MockPluginCompiler::new();
    compiler.expect_compile().returning(|_, _| Ok(()));
    compiler
        .expect_remove_compiled()
        .withf(move |hash| hash == old_hash)
        .times(1)
        .returning(|_| Ok(()));
    let mut installer = Installer::open(
        places_in(folder.path()),
        Box::new(github.client()),
        Box::new(compiler),
    )
    .expect("an installer");
    install_from(&mut installer, REPO);
    installer
        .set_part_on("example", &agent(), false)
        .expect("off");
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0"),
            FakeRelease::example("v0.2.0", "0.2.0")
                .with_file("example.wasm", &rebuilt_wasm()),
        ],
    );

    let report = installer.check_update("example");

    assert_eq!(
        *report.outcome(),
        Ok(UpdateOutcome::Updated {
            from: PluginVersion::parse("0.1.0").expect("version"),
            to: PluginVersion::parse("0.2.0").expect("version"),
        })
    );
    assert_eq!(version_of(&installer, "example"), "0.2.0");
    assert!(!part_is_on(&installer, "example", &agent()));
    assert!(part_is_on(&installer, "example", &provider()));
    assert!(!folder.path().join("data/plugins/example/0.1.0").exists());
    assert!(
        folder
            .path()
            .join("data/plugins/example/0.2.0/example.wasm")
            .exists()
    );
    let hashes = installer.plugin("example").expect("installed").hashes();
    assert_eq!(hashes.keys().collect::<Vec<_>>(), ["v0.1.0", "v0.2.0"]);
}

#[test]
fn an_update_with_new_permissions_waits_and_the_old_version_keeps_running() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0"),
            FakeRelease::example("v0.2.0", "0.2.0").with_manifest_edit(
                "http = [\"api.example.com\"]",
                "http = [\"api.example.com\", \"billing.example.com\"]",
            ),
        ],
    );

    let report = installer.check_update("example");

    let Ok(UpdateOutcome::Waiting(pending)) = report.outcome() else {
        panic!("waiting: {report:?}");
    };
    assert_eq!(
        pending.diff().parts(),
        [PartChange::Changed {
            part: provider(),
            added: vec!["Sends requests to `billing.example.com`".to_string()],
            removed: Vec::new(),
        }]
    );
    assert_eq!(version_of(&installer, "example"), "0.1.0");
    assert!(folder.path().join("data/plugins/example/0.1.0").exists());

    installer.apply_update(pending).expect("updated");

    assert_eq!(version_of(&installer, "example"), "0.2.0");
    assert!(
        installer
            .plugin("example")
            .expect("installed")
            .part(&provider())
            .expect("part")
            .granted()
            .contains(&"Sends requests to `billing.example.com`".to_string())
    );
}

#[test]
fn an_update_with_a_new_part_waits() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(
        REPO,
        vec![agent_only(FakeRelease::example("v0.1.0", "0.1.0"))],
    );
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    github.publish(
        REPO,
        vec![
            agent_only(FakeRelease::example("v0.1.0", "0.1.0")),
            FakeRelease::example("v0.2.0", "0.2.0"),
        ],
    );

    let report = installer.check_update("example");

    let Ok(UpdateOutcome::Waiting(pending)) = report.outcome() else {
        panic!("waiting: {report:?}");
    };
    assert!(matches!(
        pending.diff().parts(),
        [PartChange::New { part, .. }] if *part == provider()
    ));
    assert_eq!(version_of(&installer, "example"), "0.1.0");
}

#[test]
fn a_newer_release_that_needs_a_newer_otty_is_reported_and_skipped() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0"),
            needs_agent_0_2(FakeRelease::example("v0.6.0", "0.6.0")),
        ],
    );

    let report = installer.check_update("example");

    assert_eq!(*report.outcome(), Ok(UpdateOutcome::UpToDate));
    assert_eq!(
        report.too_new().map(|skipped| skipped.tag()),
        Some("v0.6.0")
    );
    assert_eq!(version_of(&installer, "example"), "0.1.0");
}

#[test]
fn a_changed_manifest_of_an_installed_tag_is_refused() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0")
                .with_manifest_edit("name = \"Example\"", "name = \"Changed\""),
        ],
    );

    let report = installer.check_update("example");

    let Err(error) = report.outcome() else {
        panic!("refused: {report:?}");
    };
    assert_eq!(
        error.to_string(),
        "The release v0.1.0 of Example changed after it was installed. OTTY keeps the installed copy."
    );
}

#[test]
fn a_changed_asset_of_a_recorded_tag_is_refused() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(REPO, vec![FakeRelease::example("v0.1.0", "0.1.0")]);
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, REPO);
    // OTTY recorded v0.2.0 with another `.wasm` before.
    installer.file.plugins[0].hashes.insert(
        "v0.2.0".to_string(),
        FileHashes::from([(
            "example.wasm".to_string(),
            sha256_hex(b"the first build"),
        )]),
    );
    github.publish(
        REPO,
        vec![
            FakeRelease::example("v0.1.0", "0.1.0"),
            FakeRelease::example("v0.2.0", "0.2.0")
                .with_file("example.wasm", &rebuilt_wasm()),
        ],
    );

    let report = installer.check_update("example");

    assert_eq!(
        *report.outcome(),
        Err(InstallError::ReleaseChanged {
            tag: "v0.2.0".to_string(),
            name: "Example".to_string(),
        })
    );
    assert_eq!(version_of(&installer, "example"), "0.1.0");
}

#[test]
fn the_daily_check_runs_once_a_day_and_leaves_local_plugins_out() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    github.publish(FORK, vec![fork_release("v0.1.0", "0.1.0")]);
    let local = TempDir::new();
    write_folder(local.path(), &example_files("0.1.0"));
    let mut installer = test_installer(folder.path(), &github);
    install_from(&mut installer, FORK);
    let plan = installer.find_local(local.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);

    let due_first = installer.is_update_check_due(now);
    let reports = installer.check_updates(now);

    assert!(due_first);
    assert_eq!(
        reports
            .iter()
            .map(UpdateReport::plugin_id)
            .collect::<Vec<_>>(),
        ["example-fork"]
    );
    assert!(!installer.is_update_check_due(now + Duration::from_secs(3600)));
    assert!(
        installer.is_update_check_due(now + Duration::from_secs(25 * 3600))
    );
}

#[test]
fn a_local_plugin_installs_from_a_folder_and_reload_picks_up_a_rebuilt_wasm() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let local = TempDir::new();
    write_folder(local.path(), &example_files("0.1.0"));
    let mut installer = test_installer(folder.path(), &github);

    let plan = installer.find_local(local.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    std::fs::write(local.path().join("example.wasm"), rebuilt_wasm())
        .expect("rebuild");
    let reload = installer.reload_local("example");

    assert_eq!(plan.trust(), Trust::Local);
    assert!(matches!(reload, Ok(UpdateOutcome::Updated { .. })));
    let plugin = installer.plugin("example").expect("installed");
    assert_eq!(plugin.trust(), Trust::Local);
    assert_eq!(
        plugin.wasm_sha256(),
        Some(sha256_hex(&rebuilt_wasm()).as_str())
    );
    let installed =
        std::fs::read(installer.folder_of(plugin).join("example.wasm"))
            .expect("wasm");
    assert_eq!(installed, rebuilt_wasm());
    assert!(!folder.path().join("data/plugins/example/.staging").exists());
}

#[test]
fn reload_of_a_local_plugin_that_asks_for_more_waits() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let local = TempDir::new();
    write_folder(local.path(), &example_files("0.1.0"));
    let mut installer = test_installer(folder.path(), &github);
    let plan = installer.find_local(local.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    let manifest = template_with(
        "read = [\"~/.example-agent*/**\"",
        "read = [\"~/.example-agent*/**\", \"~/.other/**\"",
    );
    std::fs::write(local.path().join(MANIFEST_FILE), manifest).expect("edit");

    let reload = installer.reload_local("example");
    let not_local = installer.reload_local("nothing");

    assert!(matches!(reload, Ok(UpdateOutcome::Waiting(_))));
    assert_eq!(
        not_local,
        Err(InstallError::NotInstalled("nothing".to_string()))
    );
}

#[test]
fn a_local_plugin_is_never_updated_on_its_own() {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let local = TempDir::new();
    write_folder(local.path(), &example_files("0.1.0"));
    let mut installer = test_installer(folder.path(), &github);
    let plan = installer.find_local(local.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    write_folder(local.path(), &example_files("0.2.0"));

    let reports = installer.check_updates(SystemTime::now());
    let on_demand = installer.check_update("example");

    assert!(reports.is_empty());
    assert_eq!(*on_demand.outcome(), Ok(UpdateOutcome::UpToDate));
    assert_eq!(version_of(&installer, "example"), "0.1.0");
}

#[test]
fn the_real_engine_compiles_the_template_and_a_wasm_that_does_not_serve_a_part_is_refused()
 {
    let folder = TempDir::new();
    let github = FakeGithub::new();
    let cache = folder.path().join("cache");
    let engine = PluginEngine::new(cache.clone()).expect("an engine");
    let mut installer = Installer::open(
        places_in(folder.path()),
        Box::new(github.client()),
        Box::new(engine),
    )
    .expect("an installer");
    let good = TempDir::new();
    write_folder(good.path(), &example_files("0.1.0"));
    let bad = TempDir::new();
    let mut files = example_files("0.1.0");
    files.insert("example.wasm".to_string(), agent_only_wasm());
    write_folder(bad.path(), &files);

    let refused = installer
        .find_local(bad.path())
        .and_then(|plan| installer.install(&plan));
    let plan = installer.find_local(good.path()).expect("a plan");
    installer.install(&plan).expect("installed");
    let compiled = std::fs::read_dir(&cache).expect("the cache").count();
    installer.remove("example").expect("removed");

    assert!(
        matches!(&refused, Err(InstallError::Compile(reason)) if reason.contains("`agents.provider` contract")),
        "{refused:?}"
    );
    assert_eq!(compiled, 1);
    assert_eq!(std::fs::read_dir(&cache).expect("the cache").count(), 0);
}

fn template_with(from: &str, to: &str) -> String {
    let text = String::from_utf8(example_files("0.1.0")[MANIFEST_FILE].clone())
        .expect("text");
    assert!(text.contains(from), "the manifest holds {from:?}");

    text.replacen(from, to, 1)
}
