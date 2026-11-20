//! New versions of installed plugins: the daily and on-demand update
//! check of releases, the Reload of a local plugin, and the plugins built
//! into OTTY (spec 2026-11-19-agent-plugins, "Updates", "Embedded
//! plugins", "Local plugins").

use std::time::{Duration, SystemTime};

use crate::choose::{SkippedRelease, list_releases, pick_release};
use crate::diff::PermissionDiff;
use crate::embedded::EmbeddedPlugin;
use crate::error::InstallError;
use crate::github::RepositoryUrl;
use crate::installer::{
    Installer, parts_of, remove_folder, version_folder, wasm_hash,
};
use crate::package::{Package, hashes_of};
use crate::plan::{PendingUpdate, UpdateOutcome, UpdateReport};
use crate::plugins_file::{
    FileHashes, InstalledPlugin, Served, Source, Winner,
};

/// How often the update check runs on its own.
pub const UPDATE_CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

impl Installer {
    /// Whether a day has passed since the last update check of
    /// [`Installer::check_updates`].
    pub fn is_update_check_due(&self, now: SystemTime) -> bool {
        let Some(last) = self.file.last_update_check else {
            return true;
        };

        seconds_of(now).saturating_sub(last) >= UPDATE_CHECK_EVERY.as_secs()
    }

    /// The daily update check: every plugin installed from releases is
    /// checked with [`Installer::check_update`]. Local and built-in copies
    /// are left out.
    pub fn check_updates(&mut self, now: SystemTime) -> Vec<UpdateReport> {
        self.file.last_update_check = Some(seconds_of(now));
        if let Err(error) = self.save() {
            log::warn!("the time of the update check was not kept: {error}");
        }

        let ids: Vec<String> = self
            .file
            .plugins
            .iter()
            .filter(|plugin| matches!(plugin.source(), Source::Release { .. }))
            .map(|plugin| plugin.id().to_string())
            .collect();

        ids.iter().map(|id| self.check_update(id)).collect()
    }

    /// Checks the releases of the plugin `plugin_id` ("Check for
    /// updates"). The newest release this OTTY can run installs on its own
    /// when it asks for no new permission and no new part; otherwise it
    /// waits for the user and the installed version keeps running.
    pub fn check_update(&mut self, plugin_id: &str) -> UpdateReport {
        let (pending, too_new) = match self.find_update(plugin_id) {
            Ok(found) => found,
            Err(error) => {
                return UpdateReport::new(plugin_id, Err(error), None);
            },
        };

        let outcome = match pending {
            None => Ok(UpdateOutcome::UpToDate),
            Some(pending) => self.update_or_wait(pending),
        };

        UpdateReport::new(plugin_id, outcome, too_new)
    }

    /// Installs a waiting update the user agreed to: next to the old
    /// version, then switches to it and removes the old one. Parts keep
    /// their on or off state and the user's choices. Each conflict of a
    /// new part whose id or host is in `use_new_for` goes to this plugin
    /// and the other plugin's part serving it is turned off; every other
    /// conflict stays with its current server and the new part is off.
    pub fn apply_update(
        &mut self,
        update: &PendingUpdate,
        use_new_for: &[Served],
    ) -> Result<(), InstallError> {
        self.switch_to(update.package(), use_new_for)
    }

    /// Reload of a local plugin: copies and checks its folder again and
    /// switches to the new copy, or waits for the user when it asks for
    /// new permissions or parts.
    pub fn reload_local(
        &mut self,
        plugin_id: &str,
    ) -> Result<UpdateOutcome, InstallError> {
        let Some(installed) = self.file.plugin(plugin_id) else {
            return Err(InstallError::NotInstalled(plugin_id.to_string()));
        };
        let Source::Local { folder } = installed.source() else {
            return Err(InstallError::NotLocal(plugin_id.to_string()));
        };

        let package = Package::of_folder(folder)?;
        let pending = self.pending_update(installed, package)?;

        self.update_or_wait(pending)
    }

    /// Installs the plugins built into OTTY, as on every start: a missing
    /// one installs with all parts on and no dialog; one the user removed
    /// stays removed; a newer copy of one that has no repository yet is
    /// installed as an update.
    pub fn install_embedded(
        &mut self,
        embedded: &[EmbeddedPlugin],
    ) -> Vec<UpdateReport> {
        let mut reports = Vec::new();

        for plugin in embedded {
            let removed = self
                .file
                .removed_built_in
                .iter()
                .any(|id| id == plugin.id());
            if removed {
                continue;
            }

            let outcome = self.install_one_embedded(plugin);
            reports.push(UpdateReport::new(plugin.id(), outcome, None));
        }

        reports
    }

    /// The newest release of the plugin `plugin_id` newer than the
    /// installed one, and the newest release skipped for needing a newer
    /// OTTY.
    fn find_update(
        &self,
        plugin_id: &str,
    ) -> Result<(Option<PendingUpdate>, Option<SkippedRelease>), InstallError>
    {
        let Some(installed) = self.file.plugin(plugin_id) else {
            return Err(InstallError::NotInstalled(plugin_id.to_string()));
        };
        let Source::Release { url } = installed.source() else {
            return Ok((None, None));
        };
        let repository = RepositoryUrl::parse(url)?;

        let releases = list_releases(self.http.as_ref(), &repository)?;
        let picked = pick_release(
            self.http.as_ref(),
            &repository,
            &releases,
            Some(installed),
            false,
        )?;

        let pending = match picked.package {
            Some(package) => Some(self.pending_update(installed, package)?),
            None => None,
        };
        Ok((pending, picked.too_new))
    }

    /// `package` as an update of `installed`, with the permission diff.
    fn pending_update(
        &self,
        installed: &InstalledPlugin,
        package: Package,
    ) -> Result<PendingUpdate, InstallError> {
        if package.id() != installed.id() {
            return Err(InstallError::OtherPlugin {
                expected: installed.id().to_string(),
                found: package.id().to_string(),
            });
        }

        let diff =
            PermissionDiff::between(installed.parts(), package.manifest());
        let conflicts = self
            .conflicts_of(package.manifest(), package.id())
            .into_iter()
            .filter(|conflict| installed.part(conflict.part()).is_none())
            .collect();

        Ok(PendingUpdate::new(
            installed.version().clone(),
            package,
            diff,
            conflicts,
        ))
    }

    /// Installs `pending` when it asks for nothing new, else leaves it
    /// waiting.
    fn update_or_wait(
        &mut self,
        pending: PendingUpdate,
    ) -> Result<UpdateOutcome, InstallError> {
        if pending.diff().asks_for_more() {
            return Ok(UpdateOutcome::Waiting(Box::new(pending)));
        }

        self.switch_to(pending.package(), &[])?;

        Ok(UpdateOutcome::Updated {
            from: pending.from().clone(),
            to: pending.to().clone(),
        })
    }

    /// Installs one built-in plugin, or its newer copy.
    fn install_one_embedded(
        &mut self,
        plugin: &EmbeddedPlugin,
    ) -> Result<UpdateOutcome, InstallError> {
        let source = match plugin.releases() {
            Some(url) => Source::Release {
                url: RepositoryUrl::parse(url)?.to_string(),
            },
            None => Source::Embedded,
        };
        let package = Package::of_files(plugin.checked_files()?, source, true)?;
        if package.id() != plugin.id() {
            return Err(InstallError::OtherPlugin {
                expected: plugin.id().to_string(),
                found: package.id().to_string(),
            });
        }

        let Some(installed) = self.file.plugin(plugin.id()) else {
            // A plugin the user installed keeps what it serves.
            self.install_package(&package, &[])?;
            return Ok(UpdateOutcome::Installed);
        };

        let is_newer_copy = *installed.source() == Source::Embedded
            && package.manifest().plugin().version() > installed.version();
        if !is_newer_copy {
            return Ok(UpdateOutcome::UpToDate);
        }
        let pending = self.pending_update(installed, package)?;
        self.update_or_wait(pending)
    }

    /// Installs `package` next to the installed version of its plugin,
    /// switches the record to it, then removes the old folder and compiled
    /// copy. Parts keep their on or off state; a new part is on unless
    /// another plugin serves one of its ids or hosts that is not in
    /// `use_new_for`.
    fn switch_to(
        &mut self,
        package: &Package,
        use_new_for: &[Served],
    ) -> Result<(), InstallError> {
        let id = package.id();
        let Some(old) = self.file.plugin(id).cloned() else {
            return Err(InstallError::NotInstalled(id.to_string()));
        };
        let manifest = package.manifest();

        let files = package.fetch(self.http.as_ref())?;
        let hashes = hashes_of(&files);
        check_same_files(&old, package, &hashes)?;
        self.put_in_place(package, &files)?;

        let mut parts = parts_of(manifest);
        for part in &mut parts {
            if let Some(old_part) = old.part(&part.part()) {
                part.on = old_part.is_on();
                continue;
            }
            for served in part.served() {
                let Some(current) = self.current_server(&served, id) else {
                    continue;
                };
                if use_new_for.contains(&served) {
                    self.turn_off_others(&served, id);
                    self.file.choose(served, Winner::Plugin(id.to_string()));
                } else {
                    part.on = false;
                    self.file.choose(served, current);
                }
            }
        }
        let gone: Vec<Served> = old
            .parts()
            .iter()
            .filter(|old_part| !manifest.parts().contains(&old_part.part()))
            .flat_map(|old_part| old_part.served())
            .collect();
        let winner = Winner::Plugin(id.to_string());
        self.file.choices.retain(|choice| {
            !(choice.winner == winner && gone.contains(&choice.served))
        });
        if let Some(record) = self.file.plugin_mut(id) {
            record.name = manifest.plugin().name().to_string();
            record.version = manifest.plugin().version().clone();
            record.tag = package.tag().to_string();
            record.wasm = manifest.plugin().wasm().to_string();
            record
                .hashes
                .insert(package.tag().to_string(), hashes.clone());
            record.parts = parts;
        }
        self.save()?;

        if old.version() != manifest.plugin().version() {
            remove_folder(&version_folder(
                &self.places.plugins_folder,
                id,
                &old.version().to_string(),
            ));
        }
        if let Some(old_hash) = old.wasm_sha256()
            && Some(old_hash) != wasm_hash(&hashes, manifest)
        {
            self.forget_compiled(old_hash);
        }

        Ok(())
    }
}

/// Refuses release files that differ from the ones recorded for the same
/// tag: a release must not change after it was installed. A local folder
/// changes on every rebuild, so it is not checked.
fn check_same_files(
    installed: &InstalledPlugin,
    package: &Package,
    hashes: &FileHashes,
) -> Result<(), InstallError> {
    if !matches!(package.source(), Source::Release { .. }) {
        return Ok(());
    }
    let Some(recorded) = installed.hashes().get(package.tag()) else {
        return Ok(());
    };

    let changed = hashes
        .iter()
        .any(|(name, hash)| recorded.get(name).is_some_and(|old| old != hash));
    if changed {
        return Err(InstallError::ReleaseChanged {
            tag: package.tag().to_string(),
            name: installed.name().to_string(),
        });
    }

    Ok(())
}

/// `time` in seconds since 1970.
fn seconds_of(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
