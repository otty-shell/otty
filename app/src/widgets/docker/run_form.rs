//! The Run container form (spec Decision 18): what the user typed, the
//! rule that auto remove forces Restart to No, what is known about the
//! image and how far a run is. Pure: `run_flow.rs` asks the engine.

use std::path::PathBuf;

use otty_docker::{
    EngineAddress, EngineInfo, ImageDefaults, MountSource, PortProtocol,
    PublishedPort, PullProgress, RestartPolicy, RunRequest, VolumeMount,
};
use otty_ui_kit::components::token_field::TokenList;

/// The restart choices in the order the form shows them.
pub(crate) const RESTART_CHOICES: [RestartPolicy; 4] = [
    RestartPolicy::No,
    RestartPolicy::OnFailure,
    RestartPolicy::UnlessStopped,
    RestartPolicy::Always,
];

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

/// One volume row as typed: source → path in the container, read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VolumeRow {
    source: String,
    target: String,
    is_read_only: bool,
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

    /// Whether nothing was typed in the row; such a row is left out.
    pub(crate) fn is_blank(&self) -> bool {
        self.source.trim().is_empty() && self.target.trim().is_empty()
    }

    /// The folder on this computer when the source is one (it starts with
    /// `/` or `~`); `None` for a volume name.
    pub(crate) fn host_path(&self) -> Option<PathBuf> {
        host_path(self.source.trim())
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

/// One change the user made in the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunEdit {
    Image(String),
    Name(String),
    PullMissing(bool),
    CommandInput(String),
    CommandSubmit,
    CommandBackspace,
    CommandRemove(usize),
    PortHost(usize, String),
    PortContainer(usize, String),
    PortProtocol(usize, PortProtocol),
    AddPort,
    RemovePort(usize),
    EnvName(usize, String),
    EnvValue(usize, String),
    AddEnv,
    RemoveEnv(usize),
    VolumeSource(usize, String),
    VolumeTarget(usize, String),
    VolumeReadOnly(usize, bool),
    AddVolume,
    RemoveVolume(usize),
    Network(String),
    Restart(RestartPolicy),
    AutoRemove(bool),
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
    /// The image reference the lookup is about.
    lookup_reference: String,
    lookup: ImageLookup,
    phase: RunPhase,
    notice: Option<String>,
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

    /// The engine error of the last attempt.
    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
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

    /// A new form on `engine` with `image` typed and `network` chosen; the
    /// pull switch is on, Restart is No.
    pub(crate) fn new(engine: EngineInfo, image: &str, network: &str) -> Self {
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
            lookup_reference: String::new(),
            lookup: ImageLookup::NotAsked,
            phase: RunPhase::Editing,
            notice: None,
        }
    }

    /// Run on another engine, with its default `network`. What was known
    /// about the image is dropped.
    pub(crate) fn set_engine(&mut self, engine: EngineInfo, network: &str) {
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
    pub(crate) fn apply(&mut self, edit: RunEdit) {
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
                    row.source = source;
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
    pub(crate) fn start_lookup(&mut self) -> Option<String> {
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
    pub(crate) fn finish_lookup(
        &mut self,
        engine: &EngineAddress,
        reference: &str,
        result: Result<Option<ImageDefaults>, String>,
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
            Err(message) => ImageLookup::Failed(message),
        };
        true
    }

    /// The image is pulled first.
    pub(crate) fn start_pull(&mut self) {
        self.notice = None;
        self.phase = RunPhase::Pulling(PullProgress::default());
    }

    /// Take the newest progress of the pull. Returns `false` when no pull
    /// runs.
    pub(crate) fn set_progress(&mut self, progress: PullProgress) -> bool {
        if !matches!(self.phase, RunPhase::Pulling(_)) {
            return false;
        }

        self.phase = RunPhase::Pulling(progress);
        true
    }

    /// The container is created and started.
    pub(crate) fn start_run(&mut self) {
        self.notice = None;
        self.phase = RunPhase::Starting;
    }

    /// The run failed: back to editing, every field as typed, the notice
    /// says why.
    pub(crate) fn fail(&mut self, notice: String) {
        self.phase = RunPhase::Editing;
        self.notice = Some(notice);
    }

    /// Hide the notice.
    pub(crate) fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    /// The create request of the typed form. Blank rows and environment
    /// rows without a name are left out; the word still being typed in the
    /// command counts. Call it once `run_checks::check` found nothing.
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
        for row in self.volumes.iter().filter(|row| !row.is_blank()) {
            let source = match row.host_path() {
                Some(path) => MountSource::HostPath(path),
                None => MountSource::Volume(row.source.trim().to_string()),
            };
            request = request.with_volume(VolumeMount::new(
                source,
                row.target.trim(),
                row.is_read_only,
            ));
        }
        let network = self.network.trim();
        if !network.is_empty() {
            request = request.with_network(network);
        }

        request
            .with_restart(self.restart)
            .with_auto_remove(self.is_auto_remove)
    }
}

/// Read a port: a number from 1 to 65535.
pub(crate) fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<u16>().ok().filter(|port| *port > 0)
}

/// The folder on this computer that `source` names: an absolute path, or
/// one under the home folder (`~`, `~/…`). `None` for a volume name.
fn host_path(source: &str) -> Option<PathBuf> {
    if source.starts_with('/') {
        return Some(PathBuf::from(source));
    }

    let rest = source.strip_prefix('~')?;
    if !rest.is_empty() && !rest.starts_with('/') {
        return None;
    }
    let home = std::env::var("HOME").unwrap_or_default();

    Some(PathBuf::from(format!("{home}{rest}")))
}

/// Remove the item at `index`; nothing past the end.
fn remove_at<T>(items: &mut Vec<T>, index: usize) {
    if index < items.len() {
        items.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn form() -> RunForm {
        RunForm::new(docker(), "nginx:1.27", "bridge")
    }

    fn edit(form: &mut RunForm, edits: Vec<RunEdit>) {
        for change in edits {
            form.apply(change);
        }
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
        let home = std::env::var("HOME").unwrap_or_default();
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
                MountSource::HostPath(PathBuf::from(format!("{home}/site"))),
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
    fn a_failure_keeps_every_field_and_shows_the_notice() {
        let mut form = form();
        form.apply(RunEdit::Name(String::from("web")));
        form.start_run();

        form.fail(String::from("port is already allocated"));

        assert!(form.is_editing());
        assert_eq!(form.name(), "web");
        assert_eq!(form.notice(), Some("port is already allocated"));
        form.dismiss_notice();
        assert_eq!(form.notice(), None);
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
            Err(String::from("store is locked")),
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
        let home = std::env::var("HOME").unwrap_or_default();

        assert_eq!(host_path("/srv/site"), Some(PathBuf::from("/srv/site")));
        assert_eq!(host_path("~"), Some(PathBuf::from(&home)));
        assert_eq!(
            host_path("~/site"),
            Some(PathBuf::from(format!("{home}/site")))
        );
        assert_eq!(host_path("otty-fx-cache"), None);
        assert_eq!(host_path("~other/site"), None);
    }
}
