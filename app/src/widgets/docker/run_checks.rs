//! The checks before Run (spec Decision 18): the image is not empty, ports
//! are 1–65535, paths in the container are absolute and a folder source
//! exists. Each problem is shown under its field and keeps Run disabled.

use super::run_form::{PortRow, RunForm, VolumeRow, parse_port};
use crate::i18n::Key;

/// The problem of one port or volume row: its message and which of the
/// two inputs is wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowProblem {
    message: Key,
    is_source: bool,
    is_target: bool,
}

impl RowProblem {
    /// The message under the row.
    pub(crate) fn message(&self) -> Key {
        self.message
    }

    /// Whether the source (host port, volume or folder) is wrong.
    pub(crate) fn is_source(&self) -> bool {
        self.is_source
    }

    /// Whether the target (container port or path) is wrong.
    pub(crate) fn is_target(&self) -> bool {
        self.is_target
    }

    fn new(message: Key, is_source: bool, is_target: bool) -> Self {
        Self {
            message,
            is_source,
            is_target,
        }
    }
}

/// Every problem of the form; empty when Run can go.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RunProblems {
    image: Option<Key>,
    ports: Vec<Option<RowProblem>>,
    volumes: Vec<Option<RowProblem>>,
}

impl RunProblems {
    /// The problem of the image field.
    pub(crate) fn image(&self) -> Option<Key> {
        self.image
    }

    /// The problem of the port row at `index`.
    pub(crate) fn port(&self, index: usize) -> Option<RowProblem> {
        self.ports.get(index).copied().flatten()
    }

    /// The problem of the volume row at `index`.
    pub(crate) fn volume(&self, index: usize) -> Option<RowProblem> {
        self.volumes.get(index).copied().flatten()
    }

    /// Whether nothing is wrong.
    pub(crate) fn is_empty(&self) -> bool {
        self.image.is_none()
            && self.ports.iter().all(Option::is_none)
            && self.volumes.iter().all(Option::is_none)
    }
}

/// Check the typed form. A folder source is looked up on disk.
pub(crate) fn check(form: &RunForm) -> RunProblems {
    let image = form
        .reference()
        .is_empty()
        .then_some(Key::DockerRunErrImage);

    RunProblems {
        image,
        ports: form.ports().iter().map(port_problem).collect(),
        volumes: form.volumes().iter().map(volume_problem).collect(),
    }
}

/// A blank row is fine. The host port may be empty (the engine picks
/// one); the container port is required.
fn port_problem(row: &PortRow) -> Option<RowProblem> {
    if row.is_blank() {
        return None;
    }

    let is_host_wrong =
        !row.host().trim().is_empty() && parse_port(row.host()).is_none();
    let is_container_wrong = parse_port(row.container()).is_none();
    if !is_host_wrong && !is_container_wrong {
        return None;
    }

    Some(RowProblem::new(
        Key::DockerRunErrPort,
        is_host_wrong,
        is_container_wrong,
    ))
}

/// A blank row is fine. The path in the container must start with `/`;
/// the source is required, and a folder must exist.
fn volume_problem(row: &VolumeRow) -> Option<RowProblem> {
    if row.is_blank() {
        return None;
    }

    if !row.target().trim().starts_with('/') {
        return Some(RowProblem::new(
            Key::DockerRunErrContainerPath,
            false,
            true,
        ));
    }
    if row.source().trim().is_empty() {
        return Some(RowProblem::new(
            Key::DockerRunErrSourceRequired,
            true,
            false,
        ));
    }
    let is_missing_folder = row.host_path().is_some_and(|path| !path.exists());
    if is_missing_folder {
        return Some(RowProblem::new(
            Key::DockerRunErrSourceMissing,
            true,
            false,
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::run_form::RunEdit;

    fn form(edits: Vec<RunEdit>) -> RunForm {
        let mut form = RunForm::new(
            fixtures::engine(EngineKind::Docker),
            "nginx:1.27",
            "bridge",
        );
        for edit in edits {
            form.apply(edit);
        }
        form
    }

    fn port(host: &str, container: &str) -> Vec<RunEdit> {
        vec![
            RunEdit::AddPort,
            RunEdit::PortHost(0, host.to_string()),
            RunEdit::PortContainer(0, container.to_string()),
        ]
    }

    fn volume(source: &str, target: &str) -> Vec<RunEdit> {
        vec![
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, source.to_string()),
            RunEdit::VolumeTarget(0, target.to_string()),
        ]
    }

    #[test]
    fn a_filled_form_has_no_problems() {
        let temp = std::env::temp_dir();
        let mut edits = port("8081", "80");
        edits.extend(volume(&temp.display().to_string(), "/data"));
        edits.push(RunEdit::AddPort);
        edits.push(RunEdit::AddVolume);

        let problems = check(&form(edits));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_image_is_required() {
        let problems = check(&form(vec![RunEdit::Image(String::from("  "))]));

        assert_eq!(problems.image(), Some(Key::DockerRunErrImage));
        assert!(!problems.is_empty());
    }

    #[test]
    fn a_host_port_out_of_range_marks_the_host_input() {
        let problems = check(&form(port("80800", "443")));

        assert_eq!(
            problems.port(0),
            Some(RowProblem::new(Key::DockerRunErrPort, true, false))
        );
    }

    #[test]
    fn a_missing_or_bad_container_port_marks_the_container_input() {
        for container in ["", "0", "web"] {
            let problems = check(&form(port("8081", container)));

            assert_eq!(
                problems.port(0),
                Some(RowProblem::new(Key::DockerRunErrPort, false, true)),
                "{container:?}"
            );
        }
    }

    #[test]
    fn an_empty_host_port_lets_the_engine_pick_one() {
        let problems = check(&form(port("", "80")));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn both_ports_wrong_mark_both_inputs() {
        let problems = check(&form(port("x", "70000")));

        assert_eq!(
            problems.port(0),
            Some(RowProblem::new(Key::DockerRunErrPort, true, true))
        );
    }

    #[test]
    fn a_path_in_the_container_must_be_absolute() {
        let problems = check(&form(volume("otty-fx-cache", "cache")));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(Key::DockerRunErrContainerPath, false, true))
        );
    }

    #[test]
    fn a_volume_row_needs_a_source() {
        let problems = check(&form(volume(" ", "/cache")));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(
                Key::DockerRunErrSourceRequired,
                true,
                false
            ))
        );
    }

    #[test]
    fn a_folder_source_must_exist() {
        let problems =
            check(&form(volume("/nonexistent/otty-fx-missing", "/data")));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(Key::DockerRunErrSourceMissing, true, false))
        );
    }

    #[test]
    fn a_named_volume_is_not_looked_up_on_disk() {
        let problems = check(&form(volume("otty-fx-cache", "/cache")));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn rows_past_the_end_have_no_problem() {
        let problems = check(&form(Vec::new()));

        assert_eq!(problems.port(3), None);
        assert_eq!(problems.volume(3), None);
    }
}
