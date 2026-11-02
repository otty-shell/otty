//! View models of the Run container form (main area) and of the image panel
//! beside it (right panel), spec Decision 18.

use std::fmt;

use otty_docker::{EngineAddress, PortProtocol, RestartPolicy};
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::components::token_field::TokenList;

use super::form_model::{PullLineViewModel, pull_amount};
use super::format::size_text;
use super::model::engine_label;
use super::run_checks::{RowProblem, check};
use super::run_flow::can_run;
use super::run_form::{ImageLookup, RESTART_CHOICES, RunForm, RunPhase};
use super::state::DockerState;
use crate::i18n::{self, Key};

/// One engine of the engine select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineChoice {
    pub(crate) address: EngineAddress,
    pub(crate) label: String,
}

impl fmt::Display for EngineChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// One restart policy of the Restart control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestartChoice {
    pub(crate) policy: RestartPolicy,
    pub(crate) label: &'static str,
}

impl fmt::Display for RestartChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

/// One port row with its problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PortRowViewModel<'a> {
    pub(crate) host: &'a str,
    pub(crate) container: &'a str,
    pub(crate) protocol: PortProtocol,
    pub(crate) problem: Option<&'static str>,
    pub(crate) is_host_invalid: bool,
    pub(crate) is_container_invalid: bool,
}

/// One volume row with its problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VolumeRowViewModel<'a> {
    pub(crate) source: &'a str,
    pub(crate) target: &'a str,
    pub(crate) is_read_only: bool,
    /// The row shows the SELinux switch: SELinux is on and the source is a
    /// folder on this computer.
    pub(crate) offers_relabel: bool,
    pub(crate) is_relabeled: bool,
    pub(crate) problem: Option<&'static str>,
    pub(crate) is_source_invalid: bool,
    pub(crate) is_target_invalid: bool,
}

/// The Run container form.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunFormViewModel<'a> {
    /// The engine error of the last attempt.
    pub(crate) notice: Option<&'a str>,
    /// The engines to pick from; empty when only one is reached.
    pub(crate) engines: Vec<EngineChoice>,
    pub(crate) engine: EngineChoice,
    pub(crate) image: &'a str,
    pub(crate) image_error: Option<&'static str>,
    /// Whether the image is here, pulled first or missing.
    pub(crate) image_hint: Option<String>,
    pub(crate) name: &'a str,
    pub(crate) pulls_missing: bool,
    pub(crate) command: &'a TokenList,
    pub(crate) command_hint: String,
    pub(crate) ports: Vec<PortRowViewModel<'a>>,
    /// "The image listens on 80/tcp." once the image is known.
    pub(crate) ports_hint: Option<String>,
    pub(crate) env: &'a [(String, String)],
    pub(crate) volumes: Vec<VolumeRowViewModel<'a>>,
    pub(crate) networks: Vec<String>,
    pub(crate) network: String,
    pub(crate) restarts: Vec<RestartChoice>,
    pub(crate) restart: RestartChoice,
    /// The choices auto remove disables.
    pub(crate) disabled_restarts: Vec<RestartChoice>,
    pub(crate) restart_hint: &'static str,
    pub(crate) is_auto_remove: bool,
    /// `false` while a run goes on: the fields take no edits.
    pub(crate) is_editable: bool,
    /// The pull line in the footer while the image is pulled.
    pub(crate) progress: Option<PullLineViewModel>,
    pub(crate) footer_hint: String,
    pub(crate) run_label: &'static str,
    pub(crate) can_run: bool,
}

/// What the typed image sets on its own, beside the form.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImagePanelViewModel {
    /// The typed image, or "Run container" while none is typed.
    pub(crate) title: String,
    /// Whether the image is on this computer.
    pub(crate) status: Option<(String, StatusTone)>,
    /// Listens on, Command, Entrypoint, Workdir.
    pub(crate) fields: Vec<(&'static str, String)>,
    /// The muted line above the fields.
    pub(crate) hint: &'static str,
    /// The engine's message of a failed lookup.
    pub(crate) error: Option<String>,
}

/// The open Run form; `None` when none is open.
pub(crate) fn run_form(state: &DockerState) -> Option<RunFormViewModel<'_>> {
    let form = state.run_form()?;
    let problems = check(form);
    let defaults = match form.lookup() {
        ImageLookup::Here(defaults) => Some(defaults),
        _ => None,
    };

    let engine = EngineChoice {
        address: form.engine().address().clone(),
        label: engine_label(form.engine()),
    };
    let engines = if state.engines().len() > 1 {
        state
            .engines()
            .iter()
            .map(|engine| EngineChoice {
                address: engine.info().address().clone(),
                label: engine_label(engine.info()),
            })
            .collect()
    } else {
        Vec::new()
    };
    let ports = form
        .ports()
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let problem = problems.port(index);
            PortRowViewModel {
                host: row.host(),
                container: row.container(),
                protocol: row.protocol(),
                problem: problem.map(|problem| i18n::t(problem.message())),
                is_host_invalid: problem.is_some_and(|p| p.is_source()),
                is_container_invalid: problem.is_some_and(|p| p.is_target()),
            }
        })
        .collect();
    let volumes = form
        .volumes()
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let problem: Option<RowProblem> = problems.volume(index);
            VolumeRowViewModel {
                source: row.source(),
                target: row.target(),
                is_read_only: row.is_read_only(),
                offers_relabel: form.offers_relabel(index),
                is_relabeled: row.is_relabeled(),
                problem: problem.map(|problem| i18n::t(problem.message())),
                is_source_invalid: problem.is_some_and(|p| p.is_source()),
                is_target_invalid: problem.is_some_and(|p| p.is_target()),
            }
        })
        .collect();
    let restarts: Vec<RestartChoice> = RESTART_CHOICES
        .iter()
        .map(|&policy| restart_choice(policy))
        .collect();
    let disabled_restarts = if form.is_auto_remove() {
        restarts
            .iter()
            .copied()
            .filter(|choice| choice.policy != RestartPolicy::No)
            .collect()
    } else {
        Vec::new()
    };
    let restart_hint = if form.is_auto_remove() {
        i18n::t(Key::DockerRestartHintAutoRemove)
    } else {
        restart_hint(form.restart())
    };
    let progress = match form.phase() {
        RunPhase::Pulling(progress) => Some(PullLineViewModel {
            text: i18n::docker_pulling(form.reference()),
            fraction: progress.fraction(),
            amount: pull_amount(progress),
        }),
        _ => None,
    };
    let footer_hint = if !form.is_editing() {
        String::new()
    } else if !problems.is_empty() {
        i18n::t(Key::DockerRunFixFields).to_string()
    } else if form.lookup() == &ImageLookup::Loading {
        i18n::t(Key::DockerRunCheckingImage).to_string()
    } else {
        i18n::docker_filled(Key::TplDockerRunReady, "engine", &engine.label)
    };
    let run_label = if form.is_editing() {
        i18n::t(Key::DockerActionRun)
    } else {
        i18n::t(Key::DockerRunStarting)
    };

    Some(RunFormViewModel {
        notice: form.notice(),
        engines,
        engine,
        image: form.image(),
        image_error: problems.image().map(i18n::t),
        image_hint: image_hint(form),
        name: form.name(),
        pulls_missing: form.pulls_missing(),
        command: form.command(),
        command_hint: command_hint(defaults.map(|d| d.command())),
        ports,
        ports_hint: defaults
            .filter(|defaults| !defaults.ports().is_empty())
            .map(|defaults| {
                i18n::docker_filled(
                    Key::TplDockerRunImageListens,
                    "ports",
                    &defaults.ports().join(", "),
                )
            }),
        env: form.env(),
        volumes,
        networks: network_names(state, form),
        network: form.network().to_string(),
        restart: restart_choice(form.restart()),
        restarts,
        disabled_restarts,
        restart_hint,
        is_auto_remove: form.is_auto_remove(),
        is_editable: form.is_editing(),
        progress,
        footer_hint,
        run_label,
        can_run: can_run(form),
    })
}

/// The image panel of the open Run form; `None` when none is open.
pub(crate) fn image_panel(state: &DockerState) -> Option<ImagePanelViewModel> {
    let form = state.run_form()?;

    let title = match form.reference() {
        "" => i18n::t(Key::DockerRunContainer).to_string(),
        reference => reference.to_string(),
    };
    let mut panel = ImagePanelViewModel {
        title,
        status: None,
        fields: Vec::new(),
        hint: i18n::t(Key::DockerRunPanelHint),
        error: None,
    };

    match form.lookup() {
        ImageLookup::NotAsked => panel.hint = i18n::t(Key::DockerRunNoImage),
        ImageLookup::Loading => {
            panel.status = Some((
                i18n::t(Key::DockerRunCheckingImage).to_string(),
                StatusTone::Neutral,
            ));
        },
        ImageLookup::Missing => {
            panel.status = Some((
                i18n::t(Key::DockerImageMissing).to_string(),
                StatusTone::Warn,
            ));
        },
        ImageLookup::Failed(message) => {
            panel.error = Some(i18n::docker_filled(
                Key::TplDockerImageLookupFailed,
                "error",
                message,
            ));
        },
        ImageLookup::Here(defaults) => {
            panel.status =
                Some((here_text(defaults.size()), StatusTone::Neutral));
            if !defaults.ports().is_empty() {
                panel.fields.push((
                    i18n::t(Key::DockerFieldListensOn),
                    defaults.ports().join(", "),
                ));
            }
            if !defaults.command().is_empty() {
                panel.fields.push((
                    i18n::t(Key::DockerFieldCommand),
                    command_text(defaults.command()),
                ));
            }
            if !defaults.entrypoint().is_empty() {
                panel.fields.push((
                    i18n::t(Key::DockerFieldEntrypoint),
                    command_text(defaults.entrypoint()),
                ));
            }
            panel.fields.push((
                i18n::t(Key::DockerFieldWorkdir),
                defaults.workdir().unwrap_or("/").to_string(),
            ));
        },
    }

    Some(panel)
}

/// The line under the image field: here (with its size), checked, or
/// missing and whether it is pulled first.
fn image_hint(form: &RunForm) -> Option<String> {
    match form.lookup() {
        ImageLookup::Here(defaults) => Some(here_text(defaults.size())),
        ImageLookup::Loading => {
            Some(i18n::t(Key::DockerRunCheckingImage).to_string())
        },
        ImageLookup::Missing if form.pulls_missing() => {
            Some(i18n::t(Key::DockerRunImagePulledFirst).to_string())
        },
        ImageLookup::Missing => {
            Some(i18n::t(Key::DockerRunImageNotPulled).to_string())
        },
        _ => None,
    }
}

/// "On this computer, 192 MB", or without the size when it is unknown.
fn here_text(size: Option<u64>) -> String {
    let size = size.map(size_text).unwrap_or_default();

    i18n::docker_filled(Key::TplDockerImageHere, "size", &size)
        .trim_end_matches([',', ' '])
        .to_string()
}

/// The hint under the command: the image's own command when known.
fn command_hint(command: Option<&[String]>) -> String {
    match command.filter(|words| !words.is_empty()) {
        Some(words) => i18n::docker_filled(
            Key::TplDockerRunCommandHint,
            "command",
            &command_text(words),
        ),
        None => i18n::t(Key::DockerRunCommandHint).to_string(),
    }
}

/// Command words as a shell would show them: a word with a blank is
/// quoted, e.g. `nginx -g "daemon off;"`.
fn command_text(words: &[String]) -> String {
    words
        .iter()
        .map(|word| {
            if word.contains(char::is_whitespace) {
                format!("\"{word}\"")
            } else {
                word.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The networks of the form's engine; the chosen one is always offered.
fn network_names(state: &DockerState, form: &RunForm) -> Vec<String> {
    let mut names: Vec<String> = state
        .engine(form.engine().address())
        .map(|engine| {
            engine
                .networks()
                .items()
                .iter()
                .map(|network| network.name().to_string())
                .collect()
        })
        .unwrap_or_default();

    let chosen = form.network();
    if !chosen.is_empty() && !names.iter().any(|name| name == chosen) {
        names.push(chosen.to_string());
    }
    names
}

/// The translated label of a restart policy.
fn restart_choice(policy: RestartPolicy) -> RestartChoice {
    let key = match policy {
        RestartPolicy::No => Key::DockerRestartNo,
        RestartPolicy::OnFailure => Key::DockerRestartOnFailure,
        RestartPolicy::UnlessStopped => Key::DockerRestartUnlessStopped,
        RestartPolicy::Always => Key::DockerRestartAlways,
    };

    RestartChoice {
        policy,
        label: i18n::t(key),
    }
}

/// What a restart policy does, under the control.
fn restart_hint(policy: RestartPolicy) -> &'static str {
    let key = match policy {
        RestartPolicy::No => Key::DockerRestartHintNo,
        RestartPolicy::OnFailure => Key::DockerRestartHintOnFailure,
        RestartPolicy::UnlessStopped => Key::DockerRestartHintUnlessStopped,
        RestartPolicy::Always => Key::DockerRestartHintAlways,
    };

    i18n::t(key)
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::run_form::RunEdit;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn lists(kind: EngineKind) -> LoadedLists {
        let resources = fixtures::resources(kind);
        LoadedLists {
            containers: Ok(resources.containers().to_vec()),
            images: Ok(resources.images().to_vec()),
            volumes: Ok(resources.volumes().to_vec()),
            networks: Ok(resources.networks().to_vec()),
        }
    }

    /// The Run form on the first of `kinds` with `nginx:1.27` typed and
    /// the engine's answer `found`.
    fn state_with(
        kinds: &[EngineKind],
        found: Option<Result<Option<otty_docker::ImageDefaults>, String>>,
    ) -> DockerState {
        let docker = fixtures::engine(kinds[0]);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        for kind in kinds {
            let _ = reduce(
                &mut state,
                DockerIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                docker.address().clone(),
                ResourceKind::Containers,
            )),
        );
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = reduce(
            &mut state,
            DockerIntent::RunEdited(RunEdit::Image(String::from("nginx:1.27"))),
        );
        if let Some(result) = found {
            let _ = reduce(
                &mut state,
                DockerIntent::ImageDefaultsLoaded {
                    engine: docker.address().clone(),
                    reference: String::from("nginx:1.27"),
                    result,
                },
            );
        }
        state
    }

    /// The form of `here()` with one volume row from `source` and SELinux
    /// on or off.
    fn with_volume_row(source: &str, is_selinux_enabled: bool) -> DockerState {
        let mut state = here();
        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, source.to_string()),
            RunEdit::VolumeTarget(0, String::from("/data")),
        ] {
            let _ = reduce(&mut state, DockerIntent::RunEdited(edit));
        }
        state
            .run_form_mut()
            .expect("the form is open")
            .set_selinux(is_selinux_enabled);

        state
    }

    #[test]
    fn a_folder_row_offers_the_relabel_switch_only_with_selinux() {
        let cases = [
            ("/srv/site", true, true),
            ("~/site", true, true),
            ("otty-data", true, false),
            ("/srv/site", false, false),
        ];

        for (source, is_selinux_enabled, expected) in cases {
            let state = with_volume_row(source, is_selinux_enabled);

            let vm = run_form(&state).expect("form");

            assert_eq!(
                vm.volumes[0].offers_relabel, expected,
                "{source} with SELinux {is_selinux_enabled}"
            );
            assert!(!vm.volumes[0].is_relabeled, "off by default");
        }
    }

    #[test]
    fn turning_the_relabel_switch_on_shows_in_the_row() {
        let mut state = with_volume_row("/srv/site", true);

        let _ = reduce(
            &mut state,
            DockerIntent::RunEdited(RunEdit::VolumeRelabel(0, true)),
        );

        let vm = run_form(&state).expect("form");
        assert!(vm.volumes[0].is_relabeled);
        assert!(
            state.run_form().expect("form").request().volumes()[0]
                .is_relabeled()
        );
    }

    fn here() -> DockerState {
        state_with(
            &[EngineKind::Docker],
            Some(Ok(Some(fixtures::image_defaults()))),
        )
    }

    fn edit(state: &mut DockerState, edit: RunEdit) {
        let _ = reduce(state, DockerIntent::RunEdited(edit));
    }

    #[test]
    fn a_ready_form_can_run_on_its_engine() {
        let state = here();

        let vm = run_form(&state).expect("form");

        assert!(vm.can_run);
        assert!(vm.is_editable);
        assert!(vm.engines.is_empty());
        assert_eq!(vm.engine.label, "Docker 29.9.1");
        assert_eq!(
            vm.footer_hint,
            "Starts in the background on Docker 29.9.1."
        );
        assert_eq!(vm.run_label, i18n::t(Key::DockerActionRun));
        assert_eq!(vm.network, "bridge");
        assert!(vm.networks.contains(&String::from("bridge")));
        assert_eq!(vm.restart.policy, RestartPolicy::No);
        assert_eq!(vm.restarts.len(), 4);
        assert!(vm.disabled_restarts.is_empty());
        assert_eq!(vm.restart_hint, i18n::t(Key::DockerRestartHintNo));
        assert_eq!(vm.progress, None);
        assert_eq!(vm.notice, None);
    }

    #[test]
    fn what_the_image_sets_shows_under_the_fields() {
        let state = here();

        let vm = run_form(&state).expect("form");

        assert_eq!(vm.image_hint.as_deref(), Some("On this computer, 192 MB"));
        assert_eq!(
            vm.command_hint,
            "Leave empty to run the image's own command: nginx -g \"daemon off;\""
        );
        assert_eq!(
            vm.ports_hint.as_deref(),
            Some("The image listens on 443/tcp, 80/tcp.")
        );
    }

    #[test]
    fn a_missing_image_says_whether_it_is_pulled_first() {
        let mut state = state_with(&[EngineKind::Docker], Some(Ok(None)));

        let pulled = run_form(&state).and_then(|vm| vm.image_hint);
        edit(&mut state, RunEdit::PullMissing(false));
        let not_pulled = run_form(&state).and_then(|vm| vm.image_hint);

        assert_eq!(
            pulled.as_deref(),
            Some(i18n::t(Key::DockerRunImagePulledFirst))
        );
        assert_eq!(
            not_pulled.as_deref(),
            Some(i18n::t(Key::DockerRunImageNotPulled))
        );
        let vm = run_form(&state).expect("form");
        assert_eq!(vm.command_hint, i18n::t(Key::DockerRunCommandHint));
        assert_eq!(vm.ports_hint, None);
    }

    #[test]
    fn while_the_image_is_checked_run_waits() {
        let state = state_with(&[EngineKind::Docker], None);

        let vm = run_form(&state).expect("form");

        assert!(!vm.can_run);
        assert_eq!(vm.footer_hint, i18n::t(Key::DockerRunCheckingImage));
        assert_eq!(
            vm.image_hint.as_deref(),
            Some(i18n::t(Key::DockerRunCheckingImage))
        );
    }

    #[test]
    fn each_problem_shows_under_its_field_and_disables_run() {
        let mut state = here();
        edit(&mut state, RunEdit::AddPort);
        edit(&mut state, RunEdit::PortHost(0, String::from("80800")));
        edit(&mut state, RunEdit::PortContainer(0, String::from("443")));
        edit(&mut state, RunEdit::AddVolume);
        edit(&mut state, RunEdit::VolumeSource(0, String::from("cache")));
        edit(&mut state, RunEdit::VolumeTarget(0, String::from("cache")));

        let vm = run_form(&state).expect("form");

        assert!(!vm.can_run);
        assert_eq!(vm.footer_hint, i18n::t(Key::DockerRunFixFields));
        let port = &vm.ports[0];
        assert_eq!(port.problem, Some(i18n::t(Key::DockerRunErrPort)));
        assert!(port.is_host_invalid && !port.is_container_invalid);
        let volume = &vm.volumes[0];
        assert_eq!(
            volume.problem,
            Some(i18n::t(Key::DockerRunErrContainerPath))
        );
        assert!(!volume.is_source_invalid && volume.is_target_invalid);
    }

    #[test]
    fn an_empty_image_is_a_problem() {
        let mut state = here();
        edit(&mut state, RunEdit::Image(String::new()));

        let vm = run_form(&state).expect("form");

        assert_eq!(vm.image_error, Some(i18n::t(Key::DockerRunErrImage)));
        assert_eq!(vm.image_hint, None);
    }

    #[test]
    fn auto_remove_disables_the_other_restart_choices_with_a_hint() {
        let mut state = here();
        edit(&mut state, RunEdit::Restart(RestartPolicy::UnlessStopped));
        let before = run_form(&state).map(|vm| vm.restart_hint);

        edit(&mut state, RunEdit::AutoRemove(true));

        let vm = run_form(&state).expect("form");
        assert_eq!(before, Some(i18n::t(Key::DockerRestartHintUnlessStopped)));
        assert!(vm.is_auto_remove);
        assert_eq!(vm.restart.policy, RestartPolicy::No);
        assert_eq!(
            vm.disabled_restarts
                .iter()
                .map(|choice| choice.policy)
                .collect::<Vec<_>>(),
            vec![
                RestartPolicy::OnFailure,
                RestartPolicy::UnlessStopped,
                RestartPolicy::Always
            ]
        );
        assert_eq!(vm.restart_hint, i18n::t(Key::DockerRestartHintAutoRemove));
    }

    #[test]
    fn a_pull_shows_its_progress_in_the_footer_and_locks_the_form() {
        let mut state = state_with(&[EngineKind::Docker], Some(Ok(None)));
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );

        let vm = run_form(&state).expect("form");

        let progress = vm.progress.expect("a pull line");
        assert_eq!(progress.text, i18n::docker_pulling("nginx:1.27"));
        assert!(progress.amount.is_some());
        assert!(!vm.is_editable);
        assert!(!vm.can_run);
        assert_eq!(vm.run_label, i18n::t(Key::DockerRunStarting));
        assert_eq!(vm.footer_hint, "");
    }

    #[test]
    fn two_engines_can_be_picked_from() {
        let state = state_with(
            &[EngineKind::Docker, EngineKind::Podman],
            Some(Ok(None)),
        );

        let vm = run_form(&state).expect("form");

        let labels: Vec<&str> = vm
            .engines
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(labels, ["Docker 29.9.1", "Podman 5.8.8"]);
    }

    #[test]
    fn no_form_has_no_view_models() {
        let state = DockerState::default();

        assert!(run_form(&state).is_none());
        assert!(image_panel(&state).is_none());
    }

    #[test]
    fn the_panel_lists_what_the_image_sets() {
        let state = here();

        let panel = image_panel(&state).expect("panel");

        assert_eq!(panel.title, "nginx:1.27");
        assert_eq!(
            panel.status,
            Some((
                String::from("On this computer, 192 MB"),
                StatusTone::Neutral
            ))
        );
        assert_eq!(
            panel.fields,
            vec![
                (
                    i18n::t(Key::DockerFieldListensOn),
                    String::from("443/tcp, 80/tcp")
                ),
                (
                    i18n::t(Key::DockerFieldCommand),
                    String::from("nginx -g \"daemon off;\"")
                ),
                (
                    i18n::t(Key::DockerFieldEntrypoint),
                    String::from("/docker-entrypoint.sh")
                ),
                (i18n::t(Key::DockerFieldWorkdir), String::from("/")),
            ]
        );
        assert_eq!(panel.hint, i18n::t(Key::DockerRunPanelHint));
        assert_eq!(panel.error, None);
    }

    #[test]
    fn the_panel_says_when_the_image_is_missing_or_unknown() {
        let missing = state_with(&[EngineKind::Docker], Some(Ok(None)));
        let failed = state_with(
            &[EngineKind::Docker],
            Some(Err(String::from("store is locked"))),
        );
        let checking = state_with(&[EngineKind::Docker], None);
        let mut empty = here();
        edit(&mut empty, RunEdit::Image(String::new()));

        let missing = image_panel(&missing).expect("panel");
        let failed = image_panel(&failed).expect("panel");
        let checking = image_panel(&checking).expect("panel");
        let empty = image_panel(&empty).expect("panel");

        assert_eq!(
            missing.status,
            Some((
                i18n::t(Key::DockerImageMissing).to_string(),
                StatusTone::Warn
            ))
        );
        assert!(missing.fields.is_empty());
        assert_eq!(
            failed.error,
            Some(i18n::docker_filled(
                Key::TplDockerImageLookupFailed,
                "error",
                "store is locked"
            ))
        );
        assert_eq!(failed.status, None);
        assert_eq!(
            checking.status,
            Some((
                i18n::t(Key::DockerRunCheckingImage).to_string(),
                StatusTone::Neutral
            ))
        );
        assert_eq!(empty.title, i18n::t(Key::DockerRunContainer));
        assert_eq!(empty.hint, i18n::t(Key::DockerRunNoImage));
        assert_eq!(empty.status, None);
    }
}
