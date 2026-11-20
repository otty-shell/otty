//! The installed plugins: install one from a release or a folder, turn a
//! part on or off, remove one (spec 2026-11-19-agent-plugins, "Install
//! and update flow"). Updates, reloads and the built-in plugins are in
//! `installer/updates.rs`.

pub(crate) mod updates;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use otty_plugin_manifest::{Manifest, PartKind, PartRef};

use crate::choose::{list_releases, pick_release};
use crate::compiler::PluginCompiler;
use crate::diff::granted_lines;
use crate::error::InstallError;
use crate::github::RepositoryUrl;
use crate::http::HttpClient;
use crate::package::{Package, hashes_of, sha256_hex};
use crate::plan::{Conflict, InstallPlan};
use crate::plugins_file::{
    Choice, FileHashes, InstalledPart, InstalledPlugin, PluginsFile, Served,
    Winner,
};

/// Where the installed plugins live on disk.
///
/// A plain parameter bag: the app passes its folders in, and tests pass
/// temporary ones, so nothing here reads the user's real folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// `~/.local/share/otty/plugins/`: one folder per plugin and version.
    pub plugins_folder: PathBuf,
    /// `~/.config/otty/plugins.json`.
    pub plugins_file: PathBuf,
}

/// The installed plugins and everything that changes them. Each change is
/// written to `plugins.json` before it returns.
pub struct Installer {
    places: Places,
    http: Box<dyn HttpClient>,
    compiler: Box<dyn PluginCompiler>,
    built_in_readers: Vec<Served>,
    file: PluginsFile,
}

impl Installer {
    /// The installed plugins.
    pub fn plugins(&self) -> &[InstalledPlugin] {
        &self.file.plugins
    }

    /// The installed plugin `id`.
    pub fn plugin(&self, id: &str) -> Option<&InstalledPlugin> {
        self.file.plugin(id)
    }

    /// The user's choice for each id or host two plugins serve.
    pub fn choices(&self) -> &[Choice] {
        &self.file.choices
    }

    /// The folder that holds the files of the installed version of
    /// `plugin`: `<plugins folder>/<id>/<version>/`.
    pub fn folder_of(&self, plugin: &InstalledPlugin) -> PathBuf {
        version_folder(
            &self.places.plugins_folder,
            plugin.id(),
            &plugin.version().to_string(),
        )
    }

    /// The installed plugins as `plugins.json` at `places` records them.
    pub fn open(
        places: Places,
        http: Box<dyn HttpClient>,
        compiler: Box<dyn PluginCompiler>,
    ) -> Result<Self, InstallError> {
        let file = PluginsFile::load(&places.plugins_file)?;

        Ok(Self {
            places,
            http,
            compiler,
            built_in_readers: Vec::new(),
            file,
        })
    }

    /// The same, knowing the ids and hosts the readers built into OTTY
    /// serve until their plugins replace them: a part that serves one of
    /// them is a conflict too.
    pub fn with_built_in_readers(mut self, served: Vec<Served>) -> Self {
        self.built_in_readers = served;

        self
    }

    /// The newest release of the GitHub repository `url` this OTTY can
    /// run, checked and ready for the install dialog. Only its manifest is
    /// downloaded. Pre-releases count only with `include_prereleases`.
    pub fn find_release(
        &self,
        url: &str,
        include_prereleases: bool,
    ) -> Result<InstallPlan, InstallError> {
        let repository = RepositoryUrl::parse(url)?;

        let releases = list_releases(self.http.as_ref(), &repository)?;
        let picked = pick_release(
            self.http.as_ref(),
            &repository,
            &releases,
            None,
            include_prereleases,
        )?;

        let Some(package) = picked.package else {
            return Err(match picked.too_new {
                Some(skipped) => {
                    InstallError::NeedsNewerOtty(skipped.reason().to_string())
                },
                None => InstallError::NoRelease,
            });
        };
        self.plan_for(package)
    }

    /// The plugin in the local folder `folder` ("Add plugin from
    /// folder…"), checked and ready for the install dialog.
    pub fn find_local(
        &self,
        folder: &Path,
    ) -> Result<InstallPlan, InstallError> {
        let package = Package::of_folder(folder)?;

        self.plan_for(package)
    }

    /// Installs the plugin of `plan`: downloads or copies its files,
    /// checks them, compiles it and records it. Each conflict whose id or
    /// host is in `use_new_for` goes to the new plugin and the other
    /// plugin's part serving it is turned off; every other conflict stays
    /// with its current server and the new part is off.
    pub fn install(
        &mut self,
        plan: &InstallPlan,
        use_new_for: &[Served],
    ) -> Result<(), InstallError> {
        let package = plan.package();
        self.check_id_is_free(package)?;

        self.install_package(package, use_new_for)
    }

    /// Turns the part `part` of the plugin `plugin_id` on or off. Turning
    /// it on makes it serve its ids and hosts: another plugin's part that
    /// serves one of them is turned off, and the choice is recorded.
    pub fn set_part_on(
        &mut self,
        plugin_id: &str,
        part: &PartRef,
        on: bool,
    ) -> Result<(), InstallError> {
        let Some(plugin) = self.file.plugin(plugin_id) else {
            return Err(InstallError::NotInstalled(plugin_id.to_string()));
        };
        let Some(installed) = plugin.part(part) else {
            return Err(InstallError::NoSuchPart {
                plugin: plugin_id.to_string(),
                part: part.clone(),
            });
        };
        let served = installed.served();

        if on {
            for served in served {
                if self.current_server(&served, plugin_id).is_some() {
                    self.turn_off_others(&served, plugin_id);
                    self.file
                        .choose(served, Winner::Plugin(plugin_id.to_string()));
                }
            }
        }
        if let Some(installed) = self
            .file
            .plugin_mut(plugin_id)
            .and_then(|plugin| plugin.part_mut(part))
        {
            installed.on = on;
        }

        self.save()
    }

    /// Removes the plugin `plugin_id`: its record, its folder and its
    /// compiled copy. A built-in plugin is not installed again. Names and
    /// hidden items in `agents.json` are not touched.
    pub fn remove(&mut self, plugin_id: &str) -> Result<(), InstallError> {
        let Some(index) = self
            .file
            .plugins
            .iter()
            .position(|plugin| plugin.id() == plugin_id)
        else {
            return Err(InstallError::NotInstalled(plugin_id.to_string()));
        };

        let removed = self.file.plugins.remove(index);
        let winner = Winner::Plugin(plugin_id.to_string());
        self.file.choices.retain(|choice| choice.winner != winner);
        if removed.is_built_in()
            && !self.file.removed_built_in.iter().any(|id| id == plugin_id)
        {
            self.file.removed_built_in.push(plugin_id.to_string());
        }
        self.save()?;

        remove_folder(&self.places.plugins_folder.join(plugin_id));
        if let Some(hash) = removed.wasm_sha256() {
            self.forget_compiled(hash);
        }

        Ok(())
    }

    /// The install plan of `package`, refused when its id is taken.
    fn plan_for(&self, package: Package) -> Result<InstallPlan, InstallError> {
        self.check_id_is_free(&package)?;

        let conflicts = self.conflicts_of(package.manifest(), package.id());

        Ok(InstallPlan::new(package, conflicts))
    }

    /// Refuses a plugin whose id is installed already.
    fn check_id_is_free(&self, package: &Package) -> Result<(), InstallError> {
        let Some(installed) = self.file.plugin(package.id()) else {
            return Ok(());
        };

        if installed.source() == package.source() {
            return Err(InstallError::AlreadyInstalled(
                package.id().to_string(),
            ));
        }
        Err(InstallError::IdTaken {
            id: package.id().to_string(),
            origin: installed.source().origin(),
        })
    }

    /// Installs `package` as a new plugin, settling its conflicts with
    /// `use_new_for`.
    fn install_package(
        &mut self,
        package: &Package,
        use_new_for: &[Served],
    ) -> Result<(), InstallError> {
        let files = package.fetch(self.http.as_ref())?;
        let hashes = hashes_of(&files);
        self.put_in_place(package, &files)?;

        let id = package.id();
        let mut parts = parts_of(package.manifest());
        for conflict in self.conflicts_of(package.manifest(), id) {
            if use_new_for.contains(conflict.served()) {
                self.turn_off_others(conflict.served(), id);
                self.file.choose(
                    conflict.served().clone(),
                    Winner::Plugin(id.to_string()),
                );
            } else {
                turn_off(&mut parts, conflict.part());
                self.file.choose(
                    conflict.served().clone(),
                    conflict.current().clone(),
                );
            }
        }
        self.file.plugins.push(InstalledPlugin {
            id: id.to_string(),
            name: package.manifest().plugin().name().to_string(),
            source: package.source().clone(),
            built_in: package.is_built_in(),
            version: package.manifest().plugin().version().clone(),
            tag: package.tag().to_string(),
            wasm: package.manifest().plugin().wasm().to_string(),
            hashes: BTreeMap::from([(package.tag().to_string(), hashes)]),
            parts,
        });

        self.save()
    }

    /// Writes `files` to the folder of the package's version through a
    /// staging folder, and compiles the `.wasm`. A version folder that is
    /// there already, as on a reload, is replaced.
    fn put_in_place(
        &self,
        package: &Package,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), InstallError> {
        let manifest = package.manifest();
        let plugin_folder = self.places.plugins_folder.join(package.id());
        let staging = plugin_folder.join(".staging");
        let target = version_folder(
            &self.places.plugins_folder,
            package.id(),
            &manifest.plugin().version().to_string(),
        );
        let disk = |doing: &str| {
            let doing = doing.to_string();
            move |error: std::io::Error| InstallError::disk(&doing, &error)
        };

        remove_folder(&staging);
        fs::create_dir_all(&staging)
            .map_err(disk("create the plugin folder"))?;
        for (name, bytes) in files {
            fs::write(staging.join(name), bytes)
                .map_err(disk("write the plugin files"))?;
        }

        let compiled = self
            .compiler
            .compile(manifest, &staging.join(manifest.plugin().wasm()));
        if let Err(reason) = compiled {
            // The engine may have compiled it before refusing its imports
            // or exports: that copy goes too.
            if let Some(bytes) = files.get(manifest.plugin().wasm()) {
                self.forget_compiled(&sha256_hex(bytes));
            }
            remove_folder(&staging);
            return Err(InstallError::Compile(reason));
        }

        if target.exists() {
            fs::remove_dir_all(&target)
                .map_err(disk("replace the plugin folder"))?;
        }
        fs::rename(&staging, &target)
            .map_err(disk("move the plugin folder into place"))
    }

    /// Each id or host of the parts of `manifest` that a part of another
    /// plugin than `plugin_id`, or a built-in reader, serves now.
    fn conflicts_of(
        &self,
        manifest: &Manifest,
        plugin_id: &str,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for part in parts_of(manifest) {
            for served in part.served() {
                if let Some(current) = self.current_server(&served, plugin_id) {
                    conflicts.push(Conflict::new(part.part(), served, current));
                }
            }
        }

        conflicts
    }

    /// Who serves `served` now, other than the plugin `except`: a part
    /// that is on, else a built-in reader.
    fn current_server(&self, served: &Served, except: &str) -> Option<Winner> {
        let plugin = self.file.plugins.iter().find(|plugin| {
            plugin.id() != except
                && plugin.parts().iter().any(|part| {
                    part.is_on()
                        && part
                            .served()
                            .iter()
                            .any(|other| other.overlaps(served))
                })
        });
        if let Some(plugin) = plugin {
            return Some(Winner::Plugin(plugin.id().to_string()));
        }

        self.built_in_readers
            .iter()
            .any(|reader| reader.overlaps(served))
            .then_some(Winner::BuiltIn)
    }

    /// Turns off every part of a plugin other than `except` that serves
    /// `served`.
    fn turn_off_others(&mut self, served: &Served, except: &str) {
        for plugin in self
            .file
            .plugins
            .iter_mut()
            .filter(|plugin| plugin.id != except)
        {
            for part in &mut plugin.parts {
                if part.served().iter().any(|other| other.overlaps(served)) {
                    part.on = false;
                }
            }
        }
    }

    /// Removes the compiled copy of a `.wasm`; a failure only leaves a
    /// file in the cache.
    fn forget_compiled(&self, wasm_sha256: &str) {
        if let Err(reason) = self.compiler.remove_compiled(wasm_sha256) {
            log::warn!(
                "a compiled plugin could not be removed from the cache: {reason}"
            );
        }
    }

    fn save(&self) -> Result<(), InstallError> {
        self.file.save(&self.places.plugins_file)
    }
}

/// The parts of `manifest` as installed records: all on, with the
/// permissions granted as the manifest lists them.
pub(crate) fn parts_of(manifest: &Manifest) -> Vec<InstalledPart> {
    manifest
        .parts()
        .into_iter()
        .map(|part| {
            let hosts = match part.kind() {
                PartKind::Provider => manifest
                    .providers()
                    .iter()
                    .find(|provider| provider.id() == part.id())
                    .map(|provider| provider.hosts().to_vec())
                    .unwrap_or_default(),
                PartKind::Agent => Vec::new(),
            };

            InstalledPart {
                kind: part.kind(),
                id: part.id().to_string(),
                on: true,
                hosts,
                granted: granted_lines(manifest, &part),
            }
        })
        .collect()
}

/// `<plugins folder>/<id>/<version>/`.
fn version_folder(plugins_folder: &Path, id: &str, version: &str) -> PathBuf {
    plugins_folder.join(id).join(version)
}

/// Turns the part `part` of `parts` off.
fn turn_off(parts: &mut [InstalledPart], part: &PartRef) {
    if let Some(installed) =
        parts.iter_mut().find(|installed| installed.part() == *part)
    {
        installed.on = false;
    }
}

/// Removes `folder` and what it holds; a failure only leaves files behind.
fn remove_folder(folder: &Path) {
    match fs::remove_dir_all(folder) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            log::warn!("{} could not be removed: {error}", folder.display());
        },
        _ => {},
    }
}

/// The SHA-256 of the `.wasm` among `hashes`.
fn wasm_hash<'a>(
    hashes: &'a FileHashes,
    manifest: &Manifest,
) -> Option<&'a str> {
    hashes.get(manifest.plugin().wasm()).map(String::as_str)
}

#[cfg(test)]
mod tests;
