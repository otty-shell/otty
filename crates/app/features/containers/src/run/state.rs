//! The Run container form (spec Decision 18): what the user typed, the
//! rule that auto remove forces Restart to No, what is known about the
//! image and the typed folders and how far a run is. Pure: `reducer.rs`
//! asks the engine and the disk.

use std::path::{Path, PathBuf};

use otty_containers::{
    EngineAddress, EngineInfo, ImageDefaults, MountSource, PortProtocol,
    PublishedPort, PullProgress, RestartPolicy, RunRequest, VolumeMount,
};
use otty_ui_kit::components::token_field::TokenList;

use super::types::RunEdit;
use crate::types::{ContainersError, PullHandle};

/// The restart choices in the order the form shows them.
pub(crate) const RESTART_CHOICES: [RestartPolicy; 4] = [
    RestartPolicy::No,
    RestartPolicy::OnFailure,
    RestartPolicy::UnlessStopped,
    RestartPolicy::Always,
];

/// The Run container form, if one is open; it replaces the table.
#[derive(Debug, Default)]
pub(crate) struct RunState {
    form: Option<RunForm>,
    /// The abort handle of the form's running pull. It lives beside the
    /// form, so a form opened in place of a pulling one leaves that pull
    /// running, and the next pull's handle replaces it, which stops it.
    pull: Option<PullHandle>,
}

impl RunState {
    /// Return the open Run container form.
    pub(crate) fn form(&self) -> Option<&RunForm> {
        self.form.as_ref()
    }

    /// Return whether the Run container form is open.
    pub(crate) fn is_open(&self) -> bool {
        self.form.is_some()
    }

    /// Return the engine the open form runs on.
    pub(crate) fn engine(&self) -> Option<&EngineInfo> {
        self.form.as_ref().map(RunForm::engine)
    }

    /// Return whether the open form has a volume row from a folder.
    pub(crate) fn has_folder_rows(&self) -> bool {
        self.form.as_ref().is_some_and(RunForm::has_folder_rows)
    }

    /// Open a form in place of the one that was open.
    pub(super) fn open_form(&mut self, form: RunForm) {
        self.form = Some(form);
    }

    /// Return the open form to change it.
    pub(super) fn form_mut(&mut self) -> Option<&mut RunForm> {
        self.form.as_mut()
    }

    /// Close the form and return it.
    pub(super) fn take_form(&mut self) -> Option<RunForm> {
        self.form.take()
    }

    /// Keep the abort handle of the pull that just started; the handle of
    /// an earlier pull is dropped, which stops that pull.
    pub(super) fn keep_pull(&mut self, pull: PullHandle) {
        self.pull = Some(pull);
    }

    /// Take the abort handle of the running pull, to stop it.
    pub(super) fn take_pull(&mut self) -> Option<PullHandle> {
        self.pull.take()
    }

    /// Forget the abort handle of the pull that ended.
    pub(super) fn forget_pull(&mut self) {
        self.pull = None;
    }
}

#[cfg(test)]
impl RunState {
    /// Return the abort handle of the running pull.
    pub(crate) fn pull(&self) -> Option<&PullHandle> {
        self.pull.as_ref()
    }
}

/// One port row as typed: host port → container port and the protocol.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PortRow {
    host: String,
    container: String,
    protocol: PortProtocol,
}

impl PortRow {
    /// The typed host port; empty lets the engine pick one.
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    /// The typed container port.
    pub(crate) fn container(&self) -> &str {
        &self.container
    }

    /// `tcp` or `udp`.
    pub(crate) fn protocol(&self) -> PortProtocol {
        self.protocol
    }

    /// Whether nothing was typed in the row; such a row is left out.
    pub(crate) fn is_blank(&self) -> bool {
        self.host.trim().is_empty() && self.container.trim().is_empty()
    }
}

/// What the disk last said about a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FolderAnswer {
    path: PathBuf,
    exists: bool,
}

/// What a typed volume source names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum SourceKind {
    /// A volume of the engine, or nothing typed yet.
    #[default]
    VolumeName,
    /// A folder on this computer.
    Folder(PathBuf),
    /// A folder under the home folder (`~`, `~/…`), but the home folder is not
    /// known, so it names no folder yet. It is not a volume name either.
    UnknownHomeFolder,
}

/// One volume row as typed: source → path in the container, read-only,
/// the SELinux relabel switch and what the disk said about a folder source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VolumeRow {
    source: String,
    /// What the source names, read when it was typed.
    source_kind: SourceKind,
    target: String,
    is_read_only: bool,
    is_relabeled: bool,
    /// The last answer. It stays while the next text of the folder is asked
    /// about, so the error does not flicker during typing.
    folder_answer: Option<FolderAnswer>,
    /// The folder a question is out about; never another folder than the
    /// one typed.
    asked_folder: Option<PathBuf>,
    /// The folder the disk took too long to answer about; never another
    /// folder than the one typed.
    slow_folder: Option<PathBuf>,
    /// Whether the last edit of the source waits for a pause in typing. The
    /// timer skips such a row once, so it never asks about half-typed text,
    /// and asks the next time if the pause got lost.
    is_typing_pause_due: bool,
}

impl VolumeRow {
    /// The typed volume name or folder.
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    /// The typed path in the container.
    pub(crate) fn target(&self) -> &str {
        &self.target
    }

    /// Whether the container can only read it.
    pub(crate) fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Whether the switch "Let the container read this folder (:z)" is on.
    /// It counts only for a folder, and only on a computer with SELinux
    /// (see [`RunForm::offers_relabel`]).
    pub(crate) fn is_relabeled(&self) -> bool {
        self.is_relabeled
    }

    /// Whether nothing was typed in the row; such a row is left out.
    pub(crate) fn is_blank(&self) -> bool {
        self.source.trim().is_empty() && self.target.trim().is_empty()
    }

    /// Whether the source is a folder and the last answer, maybe about the
    /// previous text, said it is missing.
    pub(crate) fn is_folder_missing(&self) -> bool {
        self.host_path().is_some()
            && self
                .folder_answer
                .as_ref()
                .is_some_and(|answer| !answer.exists)
    }

    /// Whether the disk took too long to answer about the typed folder and
    /// still has not answered.
    pub(crate) fn is_folder_slow(&self) -> bool {
        self.slow_folder.is_some()
    }

    /// Whether the source uses `~` and the home folder is not known.
    pub(crate) fn is_home_unknown(&self) -> bool {
        self.source_kind == SourceKind::UnknownHomeFolder
    }

    /// Whether the disk has answered about the typed folder. A row without
    /// a folder has nothing to wait for.
    fn is_folder_answered(&self) -> bool {
        let Some(path) = self.host_path() else {
            return true;
        };

        self.folder_answer
            .as_ref()
            .is_some_and(|answer| answer.path == path)
    }

    /// The folder on this computer when the source is one (it starts with
    /// `/` or `~`); `None` for a volume name and for a source under a home
    /// folder that is not known.
    pub(crate) fn host_path(&self) -> Option<PathBuf> {
        match &self.source_kind {
            SourceKind::Folder(path) => Some(path.clone()),
            _ => None,
        }
    }

    /// Take a new source; `home_folder` is what `~` stands for. A question
    /// about another folder and silence about it no longer count for this row,
    /// and a source that is not a folder forgets the answer.
    fn set_source(&mut self, source: String, home_folder: Option<&Path>) {
        self.source_kind = source_kind(source.trim(), home_folder);
        self.source = source;

        let path = self.host_path();
        if self.asked_folder != path {
            self.asked_folder = None;
        }
        if self.slow_folder != path {
            self.slow_folder = None;
        }
        if path.is_none() {
            self.folder_answer = None;
        }
    }

    /// What the create request mounts. A source under an unknown home folder
    /// stays as typed: the checks keep Run from sending it, and if it ever got
    /// out, the engine would refuse it instead of using a wrong folder or
    /// creating a volume.
    fn mount_source(&self) -> MountSource {
        let typed = self.source.trim();

        match &self.source_kind {
            SourceKind::Folder(path) => MountSource::HostPath(path.clone()),
            SourceKind::UnknownHomeFolder => {
                MountSource::HostPath(PathBuf::from(typed))
            },
            SourceKind::VolumeName => MountSource::Volume(typed.to_string()),
        }
    }
}

/// What is known about the typed image on the form's engine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ImageLookup {
    /// No image is typed.
    #[default]
    NotAsked,
    /// The engine is asked.
    Loading,
    /// The image is on this computer and sets these defaults.
    Here(ImageDefaults),
    /// The image is not on this computer.
    Missing,
    /// The engine could not say; its message.
    Failed(String),
}

/// How far the run is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum RunPhase {
    /// The user edits the form.
    #[default]
    Editing,
    /// The image is pulled first.
    Pulling(PullProgress),
    /// The container is created and started.
    Starting,
}

/// The open Run container form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunForm {
    engine: EngineInfo,
    image: String,
    name: String,
    pulls_missing: bool,
    command: TokenList,
    ports: Vec<PortRow>,
    env: Vec<(String, String)>,
    volumes: Vec<VolumeRow>,
    network: String,
    restart: RestartPolicy,
    is_auto_remove: bool,
    /// Whether SELinux is enabled, as the engine search found.
    is_selinux_enabled: bool,
    /// The home folder, as the engine search found; `~` stands for it.
    home_folder: Option<PathBuf>,
    /// The image reference the lookup is about.
    lookup_reference: String,
    lookup: ImageLookup,
    phase: RunPhase,
}

impl RunForm {
    /// The engine the container runs on.
    pub(crate) fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The typed image.
    pub(crate) fn image(&self) -> &str {
        &self.image
    }

    /// The image reference to run: the typed image without blanks around.
    pub(crate) fn reference(&self) -> &str {
        self.image.trim()
    }

    /// The typed container name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Whether a missing image is pulled first.
    pub(crate) fn pulls_missing(&self) -> bool {
        self.pulls_missing
    }

    /// The command words and the word being typed.
    pub(crate) fn command(&self) -> &TokenList {
        &self.command
    }

    /// The port rows.
    pub(crate) fn ports(&self) -> &[PortRow] {
        &self.ports
    }

    /// The environment rows, name and value.
    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// The volume rows.
    pub(crate) fn volumes(&self) -> &[VolumeRow] {
        &self.volumes
    }

    /// The chosen network; empty uses the engine's default.
    pub(crate) fn network(&self) -> &str {
        &self.network
    }

    /// The chosen restart policy.
    pub(crate) fn restart(&self) -> RestartPolicy {
        self.restart
    }

    /// Whether the container is removed when it stops.
    pub(crate) fn is_auto_remove(&self) -> bool {
        self.is_auto_remove
    }

    /// What is known about the typed image.
    pub(crate) fn lookup(&self) -> &ImageLookup {
        &self.lookup
    }

    /// How far the run is.
    pub(crate) fn phase(&self) -> &RunPhase {
        &self.phase
    }

    /// Whether the form takes edits: nothing runs.
    pub(crate) fn is_editing(&self) -> bool {
        self.phase == RunPhase::Editing
    }

    /// Whether Run pulls the image first: the engine said it is missing
    /// and the pull switch is on.
    pub(crate) fn needs_pull(&self) -> bool {
        self.lookup == ImageLookup::Missing && self.pulls_missing
    }

    /// Whether a volume row has a folder as its source.
    pub(crate) fn has_folder_rows(&self) -> bool {
        self.volumes.iter().any(|row| row.host_path().is_some())
    }

    /// Whether the disk has answered about the typed folder of every row.
    pub(crate) fn are_folders_answered(&self) -> bool {
        self.volumes.iter().all(VolumeRow::is_folder_answered)
    }

    /// Whether a row with the folder `path` has no answer about it yet. An
    /// answered folder that is only asked about again does not count.
    pub(crate) fn is_folder_unanswered(&self, path: &Path) -> bool {
        self.volumes.iter().any(|row| {
            row.host_path().as_deref() == Some(path)
                && !row.is_folder_answered()
        })
    }

    /// Whether the volume row at `index` shows the SELinux switch: the
    /// computer has SELinux and the row's source is a folder on it. A named
    /// volume never does.
    pub(crate) fn offers_relabel(&self, index: usize) -> bool {
        self.is_selinux_enabled
            && self
                .volumes
                .get(index)
                .is_some_and(|row| row.host_path().is_some())
    }

    /// A new form on `engine` with `image` typed and `network` chosen; the
    /// pull switch is on, Restart is No. `is_selinux_enabled` says whether
    /// a folder volume can be relabeled; `home_folder` is what `~` stands for.
    pub(super) fn new(
        engine: EngineInfo,
        image: &str,
        network: &str,
        is_selinux_enabled: bool,
        home_folder: Option<&Path>,
    ) -> Self {
        Self {
            engine,
            image: image.to_string(),
            name: String::new(),
            pulls_missing: true,
            command: TokenList::default(),
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            network: network.to_string(),
            restart: RestartPolicy::No,
            is_auto_remove: false,
            is_selinux_enabled,
            home_folder: home_folder.map(Path::to_path_buf),
            lookup_reference: String::new(),
            lookup: ImageLookup::NotAsked,
            phase: RunPhase::Editing,
        }
    }

    /// Run on another engine, with its default `network`. What was known
    /// about the image is dropped.
    pub(super) fn set_engine(&mut self, engine: EngineInfo, network: &str) {
        if !self.is_editing() {
            return;
        }

        self.engine = engine;
        self.network = network.to_string();
        self.lookup = ImageLookup::NotAsked;
        self.lookup_reference.clear();
    }

    /// Take one edit. Nothing changes while a run goes on. Turning auto
    /// remove on sets Restart to No; other restart choices are ignored
    /// while it is on, because the engine refuses the pair.
    pub(super) fn apply(&mut self, edit: RunEdit) {
        if !self.is_editing() {
            return;
        }

        match edit {
            RunEdit::Image(image) => self.image = image,
            RunEdit::Name(name) => self.name = name,
            RunEdit::PullMissing(is_on) => self.pulls_missing = is_on,
            RunEdit::CommandInput(input) => self.command.set_input(input),
            RunEdit::CommandSubmit => {
                let _ = self.command.submit();
            },
            RunEdit::CommandBackspace => {
                let _ = self.command.backspace();
            },
            RunEdit::CommandRemove(index) => {
                let _ = self.command.remove(index);
            },
            RunEdit::PortHost(index, host) => {
                if let Some(row) = self.ports.get_mut(index) {
                    row.host = host;
                }
            },
            RunEdit::PortContainer(index, container) => {
                if let Some(row) = self.ports.get_mut(index) {
                    row.container = container;
                }
            },
            RunEdit::PortProtocol(index, protocol) => {
                if let Some(row) = self.ports.get_mut(index) {
                    row.protocol = protocol;
                }
            },
            RunEdit::AddPort => self.ports.push(PortRow::default()),
            RunEdit::RemovePort(index) => remove_at(&mut self.ports, index),
            RunEdit::EnvName(index, name) => {
                if let Some(row) = self.env.get_mut(index) {
                    row.0 = name;
                }
            },
            RunEdit::EnvValue(index, value) => {
                if let Some(row) = self.env.get_mut(index) {
                    row.1 = value;
                }
            },
            RunEdit::AddEnv => self.env.push((String::new(), String::new())),
            RunEdit::RemoveEnv(index) => remove_at(&mut self.env, index),
            RunEdit::VolumeSource(index, source) => {
                if let Some(row) = self.volumes.get_mut(index) {
                    row.set_source(source, self.home_folder.as_deref());
                }
            },
            RunEdit::VolumeTarget(index, target) => {
                if let Some(row) = self.volumes.get_mut(index) {
                    row.target = target;
                }
            },
            RunEdit::VolumeReadOnly(index, is_read_only) => {
                if let Some(row) = self.volumes.get_mut(index) {
                    row.is_read_only = is_read_only;
                }
            },
            RunEdit::VolumeRelabel(index, is_relabeled) => {
                if let Some(row) = self.volumes.get_mut(index) {
                    row.is_relabeled = is_relabeled;
                }
            },
            RunEdit::AddVolume => self.volumes.push(VolumeRow::default()),
            RunEdit::RemoveVolume(index) => remove_at(&mut self.volumes, index),
            RunEdit::Network(network) => self.network = network,
            RunEdit::Restart(restart) => {
                if !self.is_auto_remove {
                    self.restart = restart;
                }
            },
            RunEdit::AutoRemove(is_on) => {
                self.is_auto_remove = is_on;
                if is_on {
                    self.restart = RestartPolicy::No;
                }
            },
        }
    }

    /// Start looking up the typed image. Returns the reference to ask the
    /// engine about, or `None` when no image is typed.
    pub(super) fn start_lookup(&mut self) -> Option<String> {
        let reference = self.reference().to_string();
        self.lookup_reference.clone_from(&reference);

        if reference.is_empty() {
            self.lookup = ImageLookup::NotAsked;
            return None;
        }

        self.lookup = ImageLookup::Loading;
        Some(reference)
    }

    /// Take the engine's answer about `reference` on `engine`. An answer
    /// about another image or engine is dropped (returns `false`).
    pub(super) fn finish_lookup(
        &mut self,
        engine: &EngineAddress,
        reference: &str,
        result: Result<Option<ImageDefaults>, ContainersError>,
    ) -> bool {
        let is_asked = self.engine.address() == engine
            && self.lookup_reference == reference
            && self.lookup == ImageLookup::Loading;
        if !is_asked {
            return false;
        }

        self.lookup = match result {
            Ok(Some(defaults)) => ImageLookup::Here(defaults),
            Ok(None) => ImageLookup::Missing,
            Err(err) => ImageLookup::Failed(err.to_string()),
        };
        true
    }

    /// The source of the row at `index` was typed. Returns the folder that
    /// must now wait for a pause in typing, if it still needs a question.
    pub(super) fn start_typing_wait(
        &mut self,
        index: usize,
    ) -> Option<PathBuf> {
        let path = self.folder_to_ask(index);

        if let Some(row) = self.volumes.get_mut(index) {
            row.is_typing_pause_due = path.is_some();
        }

        path
    }

    /// The source of the row at `index` was typed and a question about that
    /// folder is already out for another row, maybe a re-check that keeps no
    /// time limit. Returns the folder when this row joins that question and
    /// so needs a time limit of its own.
    pub(super) fn join_folder_question(
        &mut self,
        index: usize,
    ) -> Option<PathBuf> {
        if !self.is_editing() {
            return None;
        }

        let path = self.volumes.get(index)?.host_path()?;
        let is_asked = self.is_folder_asked(&path);
        let row = self.volumes.get_mut(index)?;
        if !is_asked || row.asked_folder.is_some() || row.is_folder_answered() {
            return None;
        }

        row.asked_folder = Some(path.clone());
        Some(path)
    }

    /// Start asking about `path` once typing stopped on it. Returns `true`
    /// when the question must go out: a row still has this folder without an
    /// answer, no question about it is out and no run goes on.
    pub(super) fn start_folder_check(&mut self, path: &Path) -> bool {
        if !self.is_editing() || !self.is_folder_unanswered(path) {
            return false;
        }

        self.mark_folder_asked(path)
    }

    /// Start asking again about the folders of all rows. Returns each folder
    /// to ask the disk about once, without those a question is out about and
    /// those still waiting for a pause in typing. Such a row is skipped only
    /// this time. What is known about a folder stays until the answer comes.
    pub(super) fn start_folder_rechecks(&mut self) -> Vec<PathBuf> {
        if !self.is_editing() {
            return Vec::new();
        }

        let mut folders = Vec::new();
        for row in &mut self.volumes {
            if row.is_typing_pause_due {
                row.is_typing_pause_due = false;
                continue;
            }
            folders.extend(row.host_path());
        }

        folders
            .into_iter()
            .filter(|path| self.mark_folder_asked(path))
            .collect()
    }

    /// Take the disk's answer about `path` for every row with that folder.
    /// An answer about a folder no row has is dropped (returns `false`).
    pub(super) fn finish_folder_check(
        &mut self,
        path: &Path,
        exists: bool,
    ) -> bool {
        let mut is_taken = false;

        for row in &mut self.volumes {
            if row.host_path().as_deref() != Some(path) {
                continue;
            }

            row.asked_folder = None;
            row.slow_folder = None;
            row.is_typing_pause_due = false;
            row.folder_answer = Some(FolderAnswer {
                path: path.to_path_buf(),
                exists,
            });
            is_taken = true;
        }

        is_taken
    }

    /// The time for an answer about `path` is over: mark each row that has
    /// this folder and still no answer. Returns `false` when no row is marked,
    /// because no question is out (the time limit is older than the text or
    /// the form) or the answer came.
    pub(super) fn mark_folder_slow(&mut self, path: &Path) -> bool {
        if !self.is_folder_asked(path) {
            return false;
        }

        let mut is_marked = false;

        for row in &mut self.volumes {
            if row.host_path().as_deref() != Some(path)
                || row.is_folder_answered()
            {
                continue;
            }

            row.slow_folder = Some(path.to_path_buf());
            is_marked = true;
        }

        is_marked
    }

    /// The image is pulled first.
    pub(super) fn start_pull(&mut self) {
        self.phase = RunPhase::Pulling(PullProgress::default());
    }

    /// Take the newest progress of the pull. Returns `false` when no pull
    /// runs.
    pub(super) fn set_progress(&mut self, progress: PullProgress) -> bool {
        if !matches!(self.phase, RunPhase::Pulling(_)) {
            return false;
        }

        self.phase = RunPhase::Pulling(progress);
        true
    }

    /// The container is created and started.
    pub(super) fn start_run(&mut self) {
        self.phase = RunPhase::Starting;
    }

    /// The run failed: back to editing, every field as typed. The reducer
    /// tells the user why.
    pub(super) fn fail(&mut self) {
        self.phase = RunPhase::Editing;
    }

    /// The create request of the typed form. Blank rows and environment
    /// rows without a name are left out; the word still being typed in the
    /// command counts. Call it once `checks::check` found nothing.
    pub(crate) fn request(&self) -> RunRequest {
        let mut request = RunRequest::new(self.reference());

        let name = self.name.trim();
        if !name.is_empty() {
            request = request.with_name(name);
        }
        let mut command = self.command.tokens().to_vec();
        let typed = self.command.input().trim();
        if !typed.is_empty() {
            command.push(typed.to_string());
        }
        request = request.with_command(command);
        for row in self.ports.iter().filter(|row| !row.is_blank()) {
            let Some(container) = parse_port(&row.container) else {
                continue;
            };
            let host = parse_port(&row.host);
            request = request.with_port(PublishedPort::new(
                host,
                container,
                row.protocol,
            ));
        }
        for (name, value) in &self.env {
            let name = name.trim();
            if !name.is_empty() {
                request = request.with_env(name, value);
            }
        }
        for (index, row) in self
            .volumes
            .iter()
            .enumerate()
            .filter(|(_, row)| !row.is_blank())
        {
            let is_relabeled = row.is_relabeled && self.offers_relabel(index);
            request = request.with_volume(
                VolumeMount::new(
                    row.mount_source(),
                    row.target.trim(),
                    row.is_read_only,
                )
                .with_relabel(is_relabeled),
            );
        }
        let network = self.network.trim();
        if !network.is_empty() {
            request = request.with_network(network);
        }

        request
            .with_restart(self.restart)
            .with_auto_remove(self.is_auto_remove)
    }

    /// The folder of the row at `index` when it still needs a question: not
    /// when the source is not a folder, has an answer or a question out, or a
    /// run goes on.
    fn folder_to_ask(&self, index: usize) -> Option<PathBuf> {
        if !self.is_editing() {
            return None;
        }

        let row = self.volumes.get(index)?;
        let path = row.host_path()?;
        if row.is_folder_answered() || self.is_folder_asked(&path) {
            return None;
        }

        Some(path)
    }

    /// Whether a question about `path` is out for a row.
    fn is_folder_asked(&self, path: &Path) -> bool {
        self.volumes
            .iter()
            .any(|row| row.asked_folder.as_deref() == Some(path))
    }

    /// Note that a question about `path` is out for every row with that
    /// folder. Returns `false` when one already was, so none is sent again.
    fn mark_folder_asked(&mut self, path: &Path) -> bool {
        let is_new = !self.is_folder_asked(path);

        for row in &mut self.volumes {
            if row.host_path().as_deref() == Some(path) {
                row.asked_folder = Some(path.to_path_buf());
            }
        }

        is_new
    }
}

#[cfg(test)]
impl RunForm {
    /// Pretend SELinux is on or off, whatever this computer has, so tests
    /// do not depend on the machine they run on.
    pub(super) fn set_selinux(&mut self, is_enabled: bool) {
        self.is_selinux_enabled = is_enabled;
    }
}

/// Read a port: a number from 1 to 65535.
pub(crate) fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<u16>().ok().filter(|port| *port > 0)
}

/// What `source` names: an absolute path or one under the home folder (`~`,
/// `~/…`) is a folder, anything else a volume name. A home folder that is not
/// absolute counts as not known: `~` would become a relative text, which the
/// engine reads as a volume name.
fn source_kind(source: &str, home_folder: Option<&Path>) -> SourceKind {
    if source.starts_with('/') {
        return SourceKind::Folder(PathBuf::from(source));
    }

    let Some(rest) = source.strip_prefix('~') else {
        return SourceKind::VolumeName;
    };
    if !rest.is_empty() && !rest.starts_with('/') {
        return SourceKind::VolumeName;
    }

    let Some(home) = home_folder.filter(|home| home.is_absolute()) else {
        return SourceKind::UnknownHomeFolder;
    };
    // All leading slashes go: `join` with an absolute path would drop `home`.
    let rest = rest.trim_start_matches('/');
    if rest.is_empty() {
        return SourceKind::Folder(home.to_path_buf());
    }

    SourceKind::Folder(home.join(rest))
}

/// Remove the item at `index`; nothing past the end.
fn remove_at<T>(items: &mut Vec<T>, index: usize) {
    if index < items.len() {
        items.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

    use super::*;

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    /// What `~` stands for in these tests, whatever this computer has.
    const HOME: &str = "/home/ann";

    fn form_at_home(home: Option<&str>) -> RunForm {
        RunForm::new(
            docker(),
            "nginx:1.27",
            "bridge",
            false,
            home.map(Path::new),
        )
    }

    fn form() -> RunForm {
        form_at_home(Some(HOME))
    }

    fn selinux_form() -> RunForm {
        RunForm::new(
            docker(),
            "nginx:1.27",
            "bridge",
            true,
            Some(Path::new(HOME)),
        )
    }

    /// A form on the home folder `home` with one row from `source` to `/data`.
    fn row_form(home: Option<&str>, source: &str) -> RunForm {
        let mut form = form_at_home(home);
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, source.to_string()),
                RunEdit::VolumeTarget(0, String::from("/data")),
            ],
        );
        form
    }

    /// The folder that `source` names with the home folder `home`.
    fn folder_of(home: Option<&str>, source: &str) -> Option<PathBuf> {
        row_form(home, source).volumes()[0].host_path()
    }

    fn edit(form: &mut RunForm, edits: Vec<RunEdit>) {
        for change in edits {
            form.apply(change);
        }
    }

    /// A form with one volume row from `source` to `/data`, with the
    /// relabel switch turned on.
    fn relabeled(mut form: RunForm, source: &str) -> RunForm {
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, source.to_string()),
                RunEdit::VolumeTarget(0, String::from("/data")),
                RunEdit::VolumeRelabel(0, true),
            ],
        );
        form
    }

    #[test]
    fn the_relabel_switch_starts_off() {
        let mut form = selinux_form();

        form.apply(RunEdit::AddVolume);

        assert!(!form.volumes()[0].is_relabeled());
    }

    #[test]
    fn the_relabel_switch_is_offered_for_a_folder_only_and_only_with_selinux() {
        let folder = relabeled(selinux_form(), "/srv/site");
        let home_folder = relabeled(selinux_form(), "~/site");
        let named = relabeled(selinux_form(), "otty-data");
        let no_selinux = relabeled(form(), "/srv/site");

        assert!(folder.offers_relabel(0));
        assert!(home_folder.offers_relabel(0));
        assert!(!named.offers_relabel(0));
        assert!(!no_selinux.offers_relabel(0));
        assert!(!folder.offers_relabel(5));
    }

    #[test]
    fn a_relabeled_folder_goes_into_the_request_with_z() {
        let request = relabeled(selinux_form(), "/srv/site").request();

        assert!(request.volumes()[0].is_relabeled());
    }

    #[test]
    fn a_named_volume_or_a_computer_without_selinux_never_relabels() {
        let named = relabeled(selinux_form(), "otty-data").request();
        let no_selinux = relabeled(form(), "/srv/site").request();
        let untouched = {
            let mut form = selinux_form();
            edit(
                &mut form,
                vec![
                    RunEdit::AddVolume,
                    RunEdit::VolumeSource(0, String::from("/srv/site")),
                    RunEdit::VolumeTarget(0, String::from("/data")),
                ],
            );
            form.request()
        };

        assert!(!named.volumes()[0].is_relabeled());
        assert!(!no_selinux.volumes()[0].is_relabeled());
        assert!(!untouched.volumes()[0].is_relabeled());
    }

    #[test]
    fn folder_rows_are_rows_with_a_folder_source() {
        let mut form = form();
        assert!(!form.has_folder_rows());

        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("otty-data")),
            ],
        );
        assert!(!form.has_folder_rows());

        form.apply(RunEdit::VolumeSource(0, String::from("~/site")));
        assert!(form.has_folder_rows());
    }

    #[test]
    fn an_answer_stays_with_its_row_when_another_row_is_removed() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::AddVolume,
                RunEdit::VolumeSource(1, String::from("/srv/site")),
            ],
        );
        assert_eq!(form.folder_to_ask(1), Some(PathBuf::from("/srv/site")));
        assert!(form.start_folder_check(Path::new("/srv/site")));
        assert!(form.finish_folder_check(Path::new("/srv/site"), false));

        form.apply(RunEdit::RemoveVolume(0));

        assert!(form.volumes()[0].is_folder_missing());
        assert!(form.are_folders_answered());
    }

    #[test]
    fn no_folder_is_asked_about_while_a_run_goes_on() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("/srv/site")),
            ],
        );
        form.start_run();

        assert_eq!(form.folder_to_ask(0), None);
        assert!(!form.start_folder_check(Path::new("/srv/site")));
        assert!(form.start_folder_rechecks().is_empty());
    }

    #[test]
    fn a_folder_is_to_be_asked_about_until_it_is_asked_or_answered() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("/srv/site")),
            ],
        );
        let site = Path::new("/srv/site");
        assert_eq!(form.folder_to_ask(0), Some(PathBuf::from(site)));
        assert!(form.is_folder_unanswered(site));

        assert!(form.start_folder_check(site));
        assert_eq!(form.folder_to_ask(0), None);
        assert!(form.is_folder_unanswered(site));

        assert!(form.finish_folder_check(site, true));
        assert_eq!(form.folder_to_ask(0), None);
        assert!(!form.is_folder_unanswered(site));
    }

    #[test]
    fn only_rows_still_without_an_answer_are_marked_slow() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("/srv/site")),
            ],
        );
        let site = Path::new("/srv/site");
        assert!(form.finish_folder_check(site, true));
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(1, String::from("/srv/site")),
            ],
        );
        assert!(form.start_folder_check(site));

        assert!(form.mark_folder_slow(site));

        assert!(!form.volumes()[0].is_folder_slow());
        assert!(form.volumes()[1].is_folder_slow());
        assert!(!form.mark_folder_slow(Path::new("/srv/other")));
    }

    #[test]
    fn a_folder_nobody_is_asking_about_is_not_marked_slow() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("/srv/site")),
            ],
        );

        assert!(!form.mark_folder_slow(Path::new("/srv/site")));

        assert!(!form.volumes()[0].is_folder_slow());
    }

    #[test]
    fn a_new_form_pulls_missing_images_and_does_not_restart() {
        let form = form();

        assert!(form.pulls_missing());
        assert_eq!(form.restart(), RestartPolicy::No);
        assert!(!form.is_auto_remove());
        assert_eq!(form.network(), "bridge");
        assert!(form.ports().is_empty());
        assert!(form.is_editing());
    }

    #[test]
    fn auto_remove_sets_restart_to_no() {
        let mut form = form();
        form.apply(RunEdit::Restart(RestartPolicy::Always));

        form.apply(RunEdit::AutoRemove(true));

        assert!(form.is_auto_remove());
        assert_eq!(form.restart(), RestartPolicy::No);
    }

    #[test]
    fn while_auto_remove_is_on_other_restart_choices_are_ignored() {
        let mut form = form();
        form.apply(RunEdit::AutoRemove(true));

        form.apply(RunEdit::Restart(RestartPolicy::UnlessStopped));

        assert_eq!(form.restart(), RestartPolicy::No);
    }

    #[test]
    fn turning_auto_remove_off_frees_the_restart_choices() {
        let mut form = form();
        form.apply(RunEdit::AutoRemove(true));

        form.apply(RunEdit::AutoRemove(false));
        form.apply(RunEdit::Restart(RestartPolicy::OnFailure));

        assert_eq!(form.restart(), RestartPolicy::OnFailure);
    }

    #[test]
    fn rows_are_added_typed_and_removed() {
        let mut form = form();

        edit(
            &mut form,
            vec![
                RunEdit::AddPort,
                RunEdit::AddPort,
                RunEdit::PortHost(1, String::from("5353")),
                RunEdit::PortContainer(1, String::from("53")),
                RunEdit::PortProtocol(1, PortProtocol::Udp),
                RunEdit::RemovePort(0),
                RunEdit::AddEnv,
                RunEdit::EnvName(0, String::from("A")),
                RunEdit::EnvValue(0, String::from("1")),
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("cache")),
                RunEdit::VolumeTarget(0, String::from("/cache")),
                RunEdit::VolumeReadOnly(0, true),
            ],
        );

        assert_eq!(form.ports().len(), 1);
        assert_eq!(form.ports()[0].host(), "5353");
        assert_eq!(form.ports()[0].container(), "53");
        assert_eq!(form.ports()[0].protocol(), PortProtocol::Udp);
        assert_eq!(form.env(), [(String::from("A"), String::from("1"))]);
        assert_eq!(form.volumes()[0].source(), "cache");
        assert_eq!(form.volumes()[0].target(), "/cache");
        assert!(form.volumes()[0].is_read_only());

        edit(
            &mut form,
            vec![
                RunEdit::RemoveEnv(0),
                RunEdit::RemoveVolume(0),
                RunEdit::RemovePort(7),
            ],
        );
        assert!(form.env().is_empty());
        assert!(form.volumes().is_empty());
        assert_eq!(form.ports().len(), 1);
    }

    #[test]
    fn command_words_are_typed_one_by_one() {
        let mut form = form();

        edit(
            &mut form,
            vec![
                RunEdit::CommandInput(String::from("sh")),
                RunEdit::CommandSubmit,
                RunEdit::CommandInput(String::from("-c")),
                RunEdit::CommandSubmit,
                RunEdit::CommandInput(String::from("x")),
                RunEdit::CommandSubmit,
                RunEdit::CommandRemove(2),
                RunEdit::CommandBackspace,
            ],
        );

        assert_eq!(form.command().tokens(), ["sh"]);
    }

    #[test]
    fn the_form_maps_to_the_create_request() {
        let mut form = form();
        edit(
            &mut form,
            vec![
                RunEdit::Image(String::from("  nginx:1.27  ")),
                RunEdit::Name(String::from(" otty-fx-web ")),
                RunEdit::CommandInput(String::from("nginx")),
                RunEdit::CommandSubmit,
                RunEdit::CommandInput(String::from("-g")),
                RunEdit::AddPort,
                RunEdit::PortHost(0, String::from("8081")),
                RunEdit::PortContainer(0, String::from("80")),
                RunEdit::AddPort,
                RunEdit::AddPort,
                RunEdit::PortContainer(2, String::from("53")),
                RunEdit::PortProtocol(2, PortProtocol::Udp),
                RunEdit::AddEnv,
                RunEdit::EnvName(0, String::from("QUIET")),
                RunEdit::EnvValue(0, String::from("1")),
                RunEdit::AddEnv,
                RunEdit::EnvValue(1, String::from("lost")),
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("/srv/site")),
                RunEdit::VolumeTarget(0, String::from("/usr/share/nginx/html")),
                RunEdit::VolumeReadOnly(0, true),
                RunEdit::AddVolume,
                RunEdit::VolumeSource(1, String::from("otty-fx-cache")),
                RunEdit::VolumeTarget(1, String::from("/cache")),
                RunEdit::AddVolume,
                RunEdit::VolumeSource(2, String::from("~/site")),
                RunEdit::VolumeTarget(2, String::from("/site")),
                RunEdit::AddVolume,
                RunEdit::Network(String::from("otty-fx-net")),
                RunEdit::Restart(RestartPolicy::UnlessStopped),
            ],
        );

        let request = form.request();

        let expected = RunRequest::new("nginx:1.27")
            .with_name("otty-fx-web")
            .with_command(vec![String::from("nginx"), String::from("-g")])
            .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
            .with_port(PublishedPort::new(None, 53, PortProtocol::Udp))
            .with_env("QUIET", "1")
            .with_volume(VolumeMount::new(
                MountSource::HostPath(PathBuf::from("/srv/site")),
                "/usr/share/nginx/html",
                true,
            ))
            .with_volume(VolumeMount::new(
                MountSource::Volume(String::from("otty-fx-cache")),
                "/cache",
                false,
            ))
            .with_volume(VolumeMount::new(
                MountSource::HostPath(PathBuf::from("/home/ann/site")),
                "/site",
                false,
            ))
            .with_network("otty-fx-net")
            .with_restart(RestartPolicy::UnlessStopped);
        assert_eq!(request, expected);
    }

    #[test]
    fn auto_remove_goes_into_the_request() {
        let mut form = form();
        form.apply(RunEdit::AutoRemove(true));

        let request = form.request();

        assert!(request.is_auto_remove());
        assert_eq!(request.restart(), RestartPolicy::No);
        assert_eq!(request.name(), None);
        assert!(request.command().is_empty());
    }

    #[test]
    fn edits_are_ignored_while_a_run_goes_on() {
        let mut form = form();
        form.start_pull();

        form.apply(RunEdit::Image(String::from("redis")));
        form.apply(RunEdit::AddPort);

        assert_eq!(form.image(), "nginx:1.27");
        assert!(form.ports().is_empty());
    }

    #[test]
    fn a_pull_reports_its_progress_and_then_the_run_starts() {
        let mut form = form();
        assert!(!form.set_progress(fixtures::pull_progress()));

        form.start_pull();
        assert!(form.set_progress(fixtures::pull_progress()));
        assert_eq!(form.phase(), &RunPhase::Pulling(fixtures::pull_progress()));

        form.start_run();
        assert_eq!(form.phase(), &RunPhase::Starting);
    }

    #[test]
    fn a_failure_goes_back_to_editing_with_every_field_kept() {
        let mut form = form();
        form.apply(RunEdit::Name(String::from("web")));
        form.start_run();

        form.fail();

        assert!(form.is_editing());
        assert_eq!(form.name(), "web");
    }

    #[test]
    fn the_lookup_follows_the_typed_image() {
        let mut form = form();

        assert_eq!(form.start_lookup().as_deref(), Some("nginx:1.27"));
        assert_eq!(form.lookup(), &ImageLookup::Loading);

        form.apply(RunEdit::Image(String::from("  ")));
        assert_eq!(form.start_lookup(), None);
        assert_eq!(form.lookup(), &ImageLookup::NotAsked);
    }

    #[test]
    fn a_lookup_answer_is_taken_only_for_the_asked_image_and_engine() {
        let mut form = form();
        let docker = docker();
        let podman = fixtures::engine(EngineKind::Podman);
        let _ = form.start_lookup();

        assert!(!form.finish_lookup(podman.address(), "nginx:1.27", Ok(None)));
        assert!(!form.finish_lookup(docker.address(), "redis", Ok(None)));
        assert_eq!(form.lookup(), &ImageLookup::Loading);

        assert!(form.finish_lookup(
            docker.address(),
            "nginx:1.27",
            Ok(Some(fixtures::image_defaults()))
        ));
        assert_eq!(
            form.lookup(),
            &ImageLookup::Here(fixtures::image_defaults())
        );
    }

    #[test]
    fn a_missing_image_is_pulled_only_when_the_switch_is_on() {
        let mut form = form();
        let docker = docker();
        let _ = form.start_lookup();
        let _ = form.finish_lookup(docker.address(), "nginx:1.27", Ok(None));

        assert_eq!(form.lookup(), &ImageLookup::Missing);
        assert!(form.needs_pull());
        form.apply(RunEdit::PullMissing(false));
        assert!(!form.needs_pull());
    }

    #[test]
    fn a_failed_lookup_keeps_the_engines_message() {
        let mut form = form();
        let docker = docker();
        let _ = form.start_lookup();

        let _ = form.finish_lookup(
            docker.address(),
            "nginx:1.27",
            Err(ContainersError::new("store is locked")),
        );

        assert_eq!(
            form.lookup(),
            &ImageLookup::Failed(String::from("store is locked"))
        );
        assert!(!form.needs_pull());
    }

    #[test]
    fn another_engine_drops_what_was_known_about_the_image() {
        let mut form = form();
        let docker = docker();
        let _ = form.start_lookup();
        let _ = form.finish_lookup(docker.address(), "nginx:1.27", Ok(None));

        form.set_engine(fixtures::engine(EngineKind::Podman), "podman");

        assert_eq!(form.engine().kind(), EngineKind::Podman);
        assert_eq!(form.network(), "podman");
        assert_eq!(form.lookup(), &ImageLookup::NotAsked);
    }

    #[test]
    fn ports_are_numbers_from_1_to_65535() {
        assert_eq!(parse_port("80"), Some(80));
        assert_eq!(parse_port(" 65535 "), Some(65535));
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("65536"), None);
        assert_eq!(parse_port("80800"), None);
        assert_eq!(parse_port("http"), None);
        assert_eq!(parse_port(""), None);
    }

    #[test]
    fn a_source_with_a_slash_or_tilde_is_a_folder() {
        let home = Some(HOME);

        assert_eq!(
            folder_of(home, "/srv/site"),
            Some(PathBuf::from("/srv/site"))
        );
        assert_eq!(folder_of(home, "~"), Some(PathBuf::from("/home/ann")));
        assert_eq!(folder_of(home, "~/"), Some(PathBuf::from("/home/ann")));
        assert_eq!(
            folder_of(home, "~/site"),
            Some(PathBuf::from("/home/ann/site"))
        );
        assert_eq!(
            folder_of(home, " ~/site "),
            Some(PathBuf::from("/home/ann/site"))
        );
        assert_eq!(folder_of(home, "otty-fx-cache"), None);
        assert_eq!(folder_of(home, "~other/site"), None);
    }

    /// `Path::join` throws away what it joins to when given an absolute path.
    #[test]
    fn a_doubled_slash_after_the_tilde_stays_under_the_home_folder() {
        assert_eq!(
            folder_of(Some(HOME), "~//etc"),
            Some(PathBuf::from("/home/ann/etc"))
        );
    }

    #[test]
    fn the_home_folder_itself_goes_out_without_a_trailing_slash() {
        for source in ["~", "~/"] {
            let folder = folder_of(Some(HOME), source).expect("a folder");

            assert_eq!(folder.display().to_string(), HOME, "{source}");
        }
    }

    #[test]
    fn a_home_folder_ending_in_a_slash_gives_a_clean_path() {
        let folder = folder_of(Some("/home/ann/"), "~/site").expect("a folder");

        assert_eq!(folder.display().to_string(), "/home/ann/site");
    }

    #[test]
    fn without_a_home_folder_a_tilde_source_names_no_folder() {
        for source in ["~", "~/", "~/site", " ~/site "] {
            let form = row_form(None, source);
            let row = &form.volumes()[0];

            assert_eq!(row.host_path(), None, "{source:?}");
            assert!(row.is_home_unknown(), "{source:?}");
        }
    }

    /// `HOME=site` would turn `~` into `site`, which the engine reads as a
    /// volume name.
    #[test]
    fn a_home_folder_that_is_not_absolute_counts_as_unknown() {
        for source in ["~", "~/site"] {
            let form = row_form(Some("relative/ann"), source);
            let row = &form.volumes()[0];

            assert_eq!(row.host_path(), None, "{source:?}");
            assert!(row.is_home_unknown(), "{source:?}");
        }
    }

    #[test]
    fn without_a_home_folder_other_sources_mean_what_they_said() {
        let folder = row_form(None, "/srv/site");
        let named = row_form(None, "otty-data");
        let other_user = row_form(None, "~other");

        assert_eq!(
            folder.volumes()[0].host_path(),
            Some(PathBuf::from("/srv/site"))
        );
        for form in [&folder, &named, &other_user] {
            assert!(!form.volumes()[0].is_home_unknown());
        }
    }

    #[test]
    fn with_a_home_folder_a_tilde_source_is_not_unknown() {
        assert!(!row_form(Some(HOME), "~/site").volumes()[0].is_home_unknown());
    }

    /// The checks keep Run from getting here, but should one ever slip, the
    /// engine gets the text as typed: not `/site` on the root, and not a named
    /// volume that the engine would make up.
    #[test]
    fn without_a_home_folder_the_request_keeps_the_source_as_typed() {
        for (source, typed) in [("~/site", "~/site"), (" ~ ", "~")] {
            let request = row_form(None, source).request();

            assert_eq!(
                request.volumes()[0].source(),
                &MountSource::HostPath(PathBuf::from(typed)),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_tilde_source_without_a_home_folder_asks_the_disk_nothing() {
        let mut form = row_form(None, "~/site");

        assert!(!form.has_folder_rows());
        assert!(form.are_folders_answered());
        assert_eq!(form.folder_to_ask(0), None);
        assert_eq!(form.start_typing_wait(0), None);
        assert!(form.start_folder_rechecks().is_empty());
    }

    #[test]
    fn a_tilde_source_without_a_home_folder_offers_no_relabel_switch() {
        let mut form =
            RunForm::new(docker(), "nginx:1.27", "bridge", true, None);
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("~/site")),
            ],
        );

        assert!(!form.offers_relabel(0));
    }

    #[test]
    fn typing_a_tilde_without_a_home_folder_forgets_the_old_folder() {
        let site = Path::new("/srv/site");
        let mut form = row_form(None, "/srv/site");
        assert!(form.finish_folder_check(site, false));
        assert!(form.volumes()[0].is_folder_missing());

        form.apply(RunEdit::VolumeSource(0, String::from("~/site")));

        assert!(!form.volumes()[0].is_folder_missing());
        assert!(!form.finish_folder_check(site, false));
    }

    #[test]
    fn the_home_folder_stays_with_the_form_on_another_engine() {
        let mut form = form_at_home(Some(HOME));

        form.set_engine(fixtures::engine(EngineKind::Podman), "podman");
        edit(
            &mut form,
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, String::from("~/site")),
            ],
        );

        assert_eq!(
            form.volumes()[0].host_path(),
            Some(PathBuf::from("/home/ann/site"))
        );
    }
}
