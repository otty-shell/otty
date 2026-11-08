//! The checks before Run (spec Decision 18): the image is not empty, ports
//! are 1–65535, paths in the container are absolute, a folder source exists
//! and answers, and `~` has a known home folder to stand for. Each problem is
//! shown under its field and keeps Run disabled. Pure: whether a folder exists
//! is the answer kept in the form.

use super::state::{ImageLookup, PortRow, RunForm, VolumeRow, parse_port};
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

    /// Whether the user can fix something. A folder that does not answer is
    /// not such a thing: only the disk can end it.
    pub(crate) fn has_fixable_problem(&self) -> bool {
        self.image.is_some()
            || self.ports.iter().any(Option::is_some)
            || self.volumes.iter().flatten().any(|problem| {
                problem.message != Key::ContainersRunErrSourceNotAnswering
            })
    }

    /// Whether nothing is wrong.
    pub(crate) fn is_empty(&self) -> bool {
        self.image.is_none()
            && self.ports.iter().all(Option::is_none)
            && self.volumes.iter().all(Option::is_none)
    }
}

/// Check the typed form. Whether a folder exists comes from the last answer
/// of the disk kept in the form; no answer yet is no problem.
pub(crate) fn check(form: &RunForm) -> RunProblems {
    let image = form
        .reference()
        .is_empty()
        .then_some(Key::ContainersRunErrImage);

    RunProblems {
        image,
        ports: form.ports().iter().map(port_problem).collect(),
        volumes: form.volumes().iter().map(volume_problem).collect(),
    }
}

/// Whether Run can be pressed: nothing runs, every check passes and both
/// the engine about the image and the disk about the folders have answered.
pub(crate) fn can_run(form: &RunForm) -> bool {
    form.is_editing()
        && form.lookup() != &ImageLookup::Loading
        && form.are_folders_answered()
        && check(form).is_empty()
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
        Key::ContainersRunErrPort,
        is_host_wrong,
        is_container_wrong,
    ))
}

/// A blank row is fine. The disk must answer about a folder in time; this goes
/// first, because the user cannot work it out alone and a missing answer may
/// be about earlier text. The path in the container must start with `/`, the
/// source is required, `~` needs a known home folder, and the last answer must
/// not say a folder is missing.
fn volume_problem(row: &VolumeRow) -> Option<RowProblem> {
    if row.is_blank() {
        return None;
    }

    if row.is_folder_slow() {
        return Some(RowProblem::new(
            Key::ContainersRunErrSourceNotAnswering,
            true,
            false,
        ));
    }
    if !row.target().trim().starts_with('/') {
        return Some(RowProblem::new(
            Key::ContainersRunErrContainerPath,
            false,
            true,
        ));
    }
    if row.source().trim().is_empty() {
        return Some(RowProblem::new(
            Key::ContainersRunErrSourceRequired,
            true,
            false,
        ));
    }
    if row.is_home_unknown() {
        return Some(RowProblem::new(
            Key::ContainersRunErrSourceHomeUnknown,
            true,
            false,
        ));
    }
    if row.is_folder_missing() {
        return Some(RowProblem::new(
            Key::ContainersRunErrSourceMissing,
            true,
            false,
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::containers::run::types::RunEdit;

    fn form_at_home(home: Option<&Path>, edits: Vec<RunEdit>) -> RunForm {
        let mut form = RunForm::new(
            fixtures::engine(EngineKind::Docker),
            "nginx:1.27",
            "bridge",
            false,
            home,
        );
        for edit in edits {
            form.apply(edit);
        }
        form
    }

    fn form(edits: Vec<RunEdit>) -> RunForm {
        form_at_home(Some(Path::new("/home/ann")), edits)
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

    /// A form with one row from the folder `source`, and the answer `exists`
    /// about that folder.
    fn answered_form(source: &str, exists: bool) -> RunForm {
        let mut form = form(volume(source, "/data"));
        assert!(form.finish_folder_check(Path::new(source), exists));
        form
    }

    #[test]
    fn a_filled_form_has_no_problems() {
        let mut edits = port("8081", "80");
        edits.extend(volume("/srv/otty-fx-site", "/data"));
        edits.push(RunEdit::AddPort);
        edits.push(RunEdit::AddVolume);
        let mut form = form(edits);
        assert!(form.finish_folder_check(Path::new("/srv/otty-fx-site"), true));

        let problems = check(&form);

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_image_is_required() {
        let problems = check(&form(vec![RunEdit::Image(String::from("  "))]));

        assert_eq!(problems.image(), Some(Key::ContainersRunErrImage));
        assert!(!problems.is_empty());
    }

    #[test]
    fn a_host_port_out_of_range_marks_the_host_input() {
        let problems = check(&form(port("80800", "443")));

        assert_eq!(
            problems.port(0),
            Some(RowProblem::new(Key::ContainersRunErrPort, true, false))
        );
    }

    #[test]
    fn a_missing_or_bad_container_port_marks_the_container_input() {
        for container in ["", "0", "web"] {
            let problems = check(&form(port("8081", container)));

            assert_eq!(
                problems.port(0),
                Some(RowProblem::new(Key::ContainersRunErrPort, false, true)),
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
            Some(RowProblem::new(Key::ContainersRunErrPort, true, true))
        );
    }

    #[test]
    fn a_path_in_the_container_must_be_absolute() {
        let problems = check(&form(volume("otty-fx-cache", "cache")));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrContainerPath,
                false,
                true
            ))
        );
    }

    #[test]
    fn a_volume_row_needs_a_source() {
        let problems = check(&form(volume(" ", "/cache")));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrSourceRequired,
                true,
                false
            ))
        );
    }

    #[test]
    fn a_folder_the_answer_calls_missing_is_a_problem() {
        // `/` is on every disk: only the answer may decide.
        let problems = check(&answered_form("/", false));

        assert_eq!(
            problems.volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrSourceMissing,
                true,
                false
            ))
        );
    }

    #[test]
    fn a_folder_the_answer_says_exists_has_no_problem() {
        let problems = check(&answered_form("/nonexistent/otty-fx", true));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_folder_without_an_answer_has_no_problem_yet() {
        let problems = check(&form(volume("/nonexistent/otty-fx", "/data")));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_missing_answer_stays_while_the_folder_text_changes() {
        let mut form = answered_form("/srv/otty-fx", false);

        form.apply(RunEdit::VolumeSource(0, String::from("/srv/otty-fx-2")));

        assert_eq!(
            check(&form).volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrSourceMissing,
                true,
                false
            ))
        );
    }

    #[test]
    fn a_folder_that_does_not_answer_is_a_problem_of_the_source() {
        let mut form = form(volume("/srv/otty-fx", "/data"));
        assert!(form.start_folder_check(Path::new("/srv/otty-fx")));

        assert!(form.mark_folder_slow(Path::new("/srv/otty-fx")));

        assert_eq!(
            check(&form).volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrSourceNotAnswering,
                true,
                false
            ))
        );
    }

    #[test]
    fn the_hint_about_a_silent_folder_goes_before_an_older_missing_answer() {
        let mut form = answered_form("/srv/otty-fx", false);
        form.apply(RunEdit::VolumeSource(0, String::from("/srv/otty-fx-2")));
        assert!(form.start_folder_check(Path::new("/srv/otty-fx-2")));

        assert!(form.mark_folder_slow(Path::new("/srv/otty-fx-2")));

        assert_eq!(
            check(&form).volume(0).map(|problem| problem.message()),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );
    }

    #[test]
    fn the_hint_about_a_silent_folder_goes_before_a_wrong_container_path() {
        let folder = Path::new("/srv/otty-fx");
        let mut form = form(volume("/srv/otty-fx", ""));
        assert!(form.start_folder_check(folder));

        assert!(form.mark_folder_slow(folder));

        assert_eq!(
            check(&form).volume(0),
            Some(RowProblem::new(
                Key::ContainersRunErrSourceNotAnswering,
                true,
                false
            ))
        );

        assert!(form.finish_folder_check(folder, true));

        assert_eq!(
            check(&form).volume(0).map(|problem| problem.message()),
            Some(Key::ContainersRunErrContainerPath)
        );
    }

    #[test]
    fn the_container_path_error_still_goes_before_a_missing_folder() {
        let mut form = answered_form("/srv/otty-fx", false);

        form.apply(RunEdit::VolumeTarget(0, String::from("cache")));

        assert_eq!(
            check(&form).volume(0).map(|problem| problem.message()),
            Some(Key::ContainersRunErrContainerPath)
        );
    }

    #[test]
    fn a_home_source_without_a_home_folder_is_a_problem_of_the_source() {
        for source in ["~", "~/", "~/site", " ~/site "] {
            let form = form_at_home(None, volume(source, "/data"));

            let problems = check(&form);

            assert_eq!(
                problems.volume(0),
                Some(RowProblem::new(
                    Key::ContainersRunErrSourceHomeUnknown,
                    true,
                    false
                )),
                "{source:?}"
            );
            assert!(problems.has_fixable_problem(), "{source:?}");
        }
    }

    #[test]
    fn a_home_source_with_a_home_folder_has_no_problem_yet() {
        let problems = check(&form(volume("~/site", "/data")));

        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn other_sources_do_not_need_a_home_folder() {
        for source in ["/srv/site", "otty-data", "~other"] {
            let form = form_at_home(None, volume(source, "/data"));

            assert!(check(&form).is_empty(), "{source:?}");
        }
    }

    #[test]
    fn the_container_path_error_goes_before_the_unknown_home_folder() {
        let form = form_at_home(None, volume("~/site", "cache"));

        assert_eq!(
            check(&form).volume(0).map(|problem| problem.message()),
            Some(Key::ContainersRunErrContainerPath)
        );
    }

    #[test]
    fn a_named_volume_forgets_the_answer_about_a_folder() {
        let mut form = answered_form("/srv/otty-fx", false);

        form.apply(RunEdit::VolumeSource(0, String::from("otty-fx-cache")));
        assert!(check(&form).is_empty());

        form.apply(RunEdit::VolumeSource(0, String::from("/srv/other")));
        assert!(check(&form).is_empty());
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
