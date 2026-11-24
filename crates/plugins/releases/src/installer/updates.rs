//! New versions of installed plugins: the daily and on-demand update
//! check of releases and the Reload of a local plugin (spec
//! 2026-11-19-agent-plugins, "Updates", "Local plugins").

use std::time::{Duration, SystemTime};

use crate::choose::{SkippedRelease, list_releases, pick_release};
use crate::diff::PermissionDiff;
use crate::error::InstallError;
use crate::github::RepositoryUrl;
use crate::installer::{
    Installer, parts_of, remove_folder, version_folder, wasm_hash,
};
use crate::package::{Package, hashes_of};
use crate::plan::{PendingUpdate, UpdateOutcome, UpdateReport};
use crate::plugins_file::{FileHashes, InstalledPlugin, Source};

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
    /// checked with [`Installer::check_update`]. Local plugins are left
    /// out.
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
    /// their on or off state, and a new part is on.
    pub fn apply_update(
        &mut self,
        update: &PendingUpdate,
    ) -> Result<(), InstallError> {
        self.switch_to(update.package())
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

    /// Reinstall of a broken plugin, whose installed manifest can not be
    /// read or which failed to load: the newest release of its repository
    /// this OTTY can run replaces the installed copy, even when it is the
    /// same version, or waits for the user when it asks for new
    /// permissions or parts. A local plugin reads its folder again, as
    /// Reload does. Parts keep their on or off state.
    pub fn reinstall(
        &mut self,
        plugin_id: &str,
    ) -> Result<UpdateOutcome, InstallError> {
        let Some(installed) = self.file.plugin(plugin_id) else {
            return Err(InstallError::NotInstalled(plugin_id.to_string()));
        };
        let Source::Release { url } = installed.source() else {
            return self.reload_local(plugin_id);
        };
        let repository = RepositoryUrl::parse(url)?;

        let releases = list_releases(self.http.as_ref(), &repository)?;
        let picked = pick_release(
            self.http.as_ref(),
            &repository,
            &releases,
            None,
            false,
        )?;

        let Some(package) = picked.package else {
            return Err(match picked.too_new {
                Some(skipped) => {
                    InstallError::NeedsNewerOtty(skipped.reason().to_string())
                },
                None => InstallError::NoRelease,
            });
        };
        let pending = self.pending_update(installed, package)?;
        self.update_or_wait(pending)
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

        Ok(PendingUpdate::new(
            installed.version().clone(),
            package,
            diff,
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

        self.switch_to(pending.package())?;

        Ok(UpdateOutcome::Updated {
            from: pending.from().clone(),
            to: pending.to().clone(),
        })
    }

    /// Installs `package` next to the installed version of its plugin,
    /// switches the record to it, then removes the old folder and compiled
    /// copy. Parts keep their on or off state; a new part is on.
    fn switch_to(&mut self, package: &Package) -> Result<(), InstallError> {
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
            }
        }
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
