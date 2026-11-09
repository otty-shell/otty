//! View models of the Run container form (main area) and of the image panel
//! beside it (right panel), spec Decision 18.

use std::fmt;

use otty_containers::{EngineAddress, PortProtocol, RestartPolicy};
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::components::token_field::TokenList;

use super::super::super::format::{engine_label, pull_amount, size_text};
use super::checks::{RowProblem, can_run, check};
use super::reducer::RunCtx;
use super::state::{ImageLookup, RESTART_CHOICES, RunForm, RunPhase, RunState};
use crate::components::pull_line::PullLineViewModel;
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
pub(crate) struct RunFormPortRowViewModel<'a> {
    pub(crate) host: &'a str,
    pub(crate) container: &'a str,
    pub(crate) protocol: PortProtocol,
    pub(crate) problem: Option<&'static str>,
    pub(crate) is_host_invalid: bool,
    pub(crate) is_container_invalid: bool,
}

/// One volume row with its problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunFormVolumeRowViewModel<'a> {
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
    pub(crate) ports: Vec<RunFormPortRowViewModel<'a>>,
    /// "The image listens on 80/tcp." once the image is known.
    pub(crate) ports_hint: Option<String>,
    pub(crate) env: &'a [(String, String)],
    pub(crate) volumes: Vec<RunFormVolumeRowViewModel<'a>>,
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

impl<'a> RunFormViewModel<'a> {
    /// Build the open Run form; `None` when none is open, or while no engine
    /// is there: the form waits, as typed, behind the no-engine card.
    pub(crate) fn from_state(
        state: &'a RunState,
        ctx: &RunCtx<'_>,
    ) -> Option<Self> {
        if ctx.engines.is_empty() {
            return None;
        }
        let form = state.form()?;
        let problems = check(form);
        let defaults = match form.lookup() {
            ImageLookup::Here(defaults) => Some(defaults),
            _ => None,
        };

        let engine = EngineChoice {
            address: form.engine().address().clone(),
            label: engine_label(form.engine()),
        };
        let engines = if ctx.engines.len() > 1 {
            ctx.engines
                .iter()
                .map(|engine| EngineChoice {
                    address: engine.info.address().clone(),
                    label: engine_label(engine.info),
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
                RunFormPortRowViewModel {
                    host: row.host(),
                    container: row.container(),
                    protocol: row.protocol(),
                    problem: problem.map(|problem| i18n::t(problem.message())),
                    is_host_invalid: problem.is_some_and(|p| p.is_source()),
                    is_container_invalid: problem
                        .is_some_and(|p| p.is_target()),
                }
            })
            .collect();
        let volumes = form
            .volumes()
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let problem: Option<RowProblem> = problems.volume(index);
                RunFormVolumeRowViewModel {
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
            i18n::t(Key::ContainersRestartHintAutoRemove)
        } else {
            restart_hint(form.restart())
        };
        let progress = match form.phase() {
            RunPhase::Pulling(progress) => Some(PullLineViewModel {
                text: i18n::containers_pulling(form.reference()),
                fraction: progress.fraction(),
                amount: pull_amount(progress),
            }),
            _ => None,
        };
        let footer_hint = if !form.is_editing() {
            String::new()
        } else if problems.has_fixable_problem() {
            i18n::t(Key::ContainersRunFixFields).to_string()
        } else if form.lookup() == &ImageLookup::Loading {
            i18n::t(Key::ContainersRunCheckingImage).to_string()
        } else if !form.are_folders_answered() {
            i18n::t(Key::ContainersRunWaitingForFolder).to_string()
        } else {
            i18n::containers_filled(
                Key::TplContainersRunReady,
                "engine",
                &engine.label,
            )
        };
        let run_label = if form.is_editing() {
            i18n::t(Key::ContainersActionRun)
        } else {
            i18n::t(Key::ContainersRunStarting)
        };

        Some(Self {
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
                    i18n::containers_filled(
                        Key::TplContainersRunImageListens,
                        "ports",
                        &defaults.ports().join(", "),
                    )
                }),
            env: form.env(),
            volumes,
            networks: network_names(ctx, form),
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
}

/// What the typed image sets on its own, beside the form.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunPanelViewModel {
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

impl RunPanelViewModel {
    /// Build the image panel of the open Run form; `None` when none is open,
    /// or while no engine is there.
    pub(crate) fn from_state(
        state: &RunState,
        ctx: &RunCtx<'_>,
    ) -> Option<Self> {
        if ctx.engines.is_empty() {
            return None;
        }
        let form = state.form()?;

        let title = match form.reference() {
            "" => i18n::t(Key::ContainersRunContainer).to_string(),
            reference => reference.to_string(),
        };
        let mut panel = Self {
            title,
            status: None,
            fields: Vec::new(),
            hint: i18n::t(Key::ContainersRunPanelHint),
            error: None,
        };

        match form.lookup() {
            ImageLookup::NotAsked => {
                panel.hint = i18n::t(Key::ContainersRunNoImage)
            },
            ImageLookup::Loading => {
                panel.status = Some((
                    i18n::t(Key::ContainersRunCheckingImage).to_string(),
                    StatusTone::Neutral,
                ));
            },
            ImageLookup::Missing => {
                panel.status = Some((
                    i18n::t(Key::ContainersImageMissing).to_string(),
                    StatusTone::Warn,
                ));
            },
            ImageLookup::Failed(message) => {
                panel.error = Some(i18n::containers_filled(
                    Key::TplContainersImageLookupFailed,
                    "error",
                    message,
                ));
            },
            ImageLookup::Here(defaults) => {
                panel.status =
                    Some((here_text(defaults.size()), StatusTone::Neutral));
                if !defaults.ports().is_empty() {
                    panel.fields.push((
                        i18n::t(Key::ContainersFieldListensOn),
                        defaults.ports().join(", "),
                    ));
                }
                if !defaults.command().is_empty() {
                    panel.fields.push((
                        i18n::t(Key::ContainersFieldCommand),
                        command_text(defaults.command()),
                    ));
                }
                if !defaults.entrypoint().is_empty() {
                    panel.fields.push((
                        i18n::t(Key::ContainersFieldEntrypoint),
                        command_text(defaults.entrypoint()),
                    ));
                }
                panel.fields.push((
                    i18n::t(Key::ContainersFieldWorkdir),
                    defaults.workdir().unwrap_or("/").to_string(),
                ));
            },
        }

        Some(panel)
    }
}

/// The line under the image field: here (with its size), checked, or
/// missing and whether it is pulled first.
fn image_hint(form: &RunForm) -> Option<String> {
    match form.lookup() {
        ImageLookup::Here(defaults) => Some(here_text(defaults.size())),
        ImageLookup::Loading => {
            Some(i18n::t(Key::ContainersRunCheckingImage).to_string())
        },
        ImageLookup::Missing if form.pulls_missing() => {
            Some(i18n::t(Key::ContainersRunImagePulledFirst).to_string())
        },
        ImageLookup::Missing => {
            Some(i18n::t(Key::ContainersRunImageNotPulled).to_string())
        },
        _ => None,
    }
}

/// "On this computer, 192 MB", or without the size when it is unknown.
fn here_text(size: Option<u64>) -> String {
    let size = size.map(size_text).unwrap_or_default();

    i18n::containers_filled(Key::TplContainersImageHere, "size", &size)
        .trim_end_matches([',', ' '])
        .to_string()
}

/// The hint under the command: the image's own command when known.
fn command_hint(command: Option<&[String]>) -> String {
    match command.filter(|words| !words.is_empty()) {
        Some(words) => i18n::containers_filled(
            Key::TplContainersRunCommandHint,
            "command",
            &command_text(words),
        ),
        None => i18n::t(Key::ContainersRunCommandHint).to_string(),
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
fn network_names(ctx: &RunCtx<'_>, form: &RunForm) -> Vec<String> {
    let mut names: Vec<String> = ctx
        .engine(form.engine().address())
        .map(|engine| {
            engine
                .networks
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
        RestartPolicy::No => Key::ContainersRestartNo,
        RestartPolicy::OnFailure => Key::ContainersRestartOnFailure,
        RestartPolicy::UnlessStopped => Key::ContainersRestartUnlessStopped,
        RestartPolicy::Always => Key::ContainersRestartAlways,
    };

    RestartChoice {
        policy,
        label: i18n::t(key),
    }
}

/// What a restart policy does, under the control.
fn restart_hint(policy: RestartPolicy) -> &'static str {
    let key = match policy {
        RestartPolicy::No => Key::ContainersRestartHintNo,
        RestartPolicy::OnFailure => Key::ContainersRestartHintOnFailure,
        RestartPolicy::UnlessStopped => Key::ContainersRestartHintUnlessStopped,
        RestartPolicy::Always => Key::ContainersRestartHintAlways,
    };

    i18n::t(key)
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::containers::child::run::event::RunIntent;
    use crate::widgets::containers::child::run::types::RunEdit;
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::reducer::reduce as reduce_containers;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{
        ContainersError, LoadedLists, Place, ResourceKind,
    };

    /// The Run form of `state` as the widget builds it.
    fn run_form(state: &ContainersState) -> Option<RunFormViewModel<'_>> {
        RunFormViewModel::from_state(state.run(), &state.run_ctx())
    }

    /// The image panel of `state` as the widget builds it.
    fn image_panel(state: &ContainersState) -> Option<RunPanelViewModel> {
        RunPanelViewModel::from_state(state.run(), &state.run_ctx())
    }

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
        found: Option<
            Result<Option<otty_containers::ImageDefaults>, ContainersError>,
        >,
    ) -> ContainersState {
        let docker = fixtures::engine(kinds[0]);
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        for kind in kinds {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker.address().clone(),
                ResourceKind::Containers,
            )),
        );
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Edited(RunEdit::Image(
                String::from("nginx:1.27"),
            ))),
        );
        if let Some(result) = found {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Run(RunIntent::ImageDefaultsLoaded {
                    engine: docker.address().clone(),
                    reference: String::from("nginx:1.27"),
                    result,
                }),
            );
        }
        state
    }

    /// The form of `here()` with one volume row from `source` and SELinux
    /// on or off.
    fn with_volume_row(
        source: &str,
        is_selinux_enabled: bool,
    ) -> ContainersState {
        let mut state = here();
        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, source.to_string()),
            RunEdit::VolumeTarget(0, String::from("/data")),
        ] {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Run(RunIntent::Edited(edit)),
            );
        }
        let (run, _) = state.run_and_ctx();
        run.form_mut()
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
    fn a_home_source_without_a_home_folder_shows_its_message_under_the_row() {
        let kinds = [EngineKind::Docker];
        let docker = fixtures::engine(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(
                fixtures::discovery_without_home(&kinds),
            ),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker.address().clone(),
                ResourceKind::Containers,
            )),
        );
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        for edit in [
            RunEdit::Image(String::from("nginx:1.27")),
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, String::from("~/site")),
            RunEdit::VolumeTarget(0, String::from("/data")),
        ] {
            let _ = reduce_containers(
                &mut state,
                ContainersIntent::Run(RunIntent::Edited(edit)),
            );
        }

        let vm = run_form(&state).expect("form");

        assert!(!vm.can_run);
        assert_eq!(
            vm.volumes[0].problem,
            Some(i18n::t(Key::ContainersRunErrSourceHomeUnknown))
        );
        assert!(vm.volumes[0].is_source_invalid);
        assert!(!vm.volumes[0].is_target_invalid);
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunFixFields));
    }

    #[test]
    fn a_folder_being_asked_about_disables_run_without_a_message() {
        let state = with_volume_row("/srv/site", false);

        let vm = run_form(&state).expect("form");

        assert!(!vm.can_run);
        assert_eq!(vm.volumes[0].problem, None);
        assert!(!vm.volumes[0].is_source_invalid);
    }

    #[test]
    fn a_folder_answered_missing_shows_its_message_under_the_row() {
        let mut state = with_volume_row("/srv/site", false);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::FolderChecked {
                path: std::path::PathBuf::from("/srv/site"),
                exists: false,
            }),
        );

        let vm = run_form(&state).expect("form");
        assert!(!vm.can_run);
        assert_eq!(
            vm.volumes[0].problem,
            Some(i18n::t(Key::ContainersRunErrSourceMissing))
        );
        assert!(vm.volumes[0].is_source_invalid);
    }

    #[test]
    fn a_folder_that_does_not_answer_shows_its_hint_under_the_row() {
        let mut state = with_volume_row("/srv/site", false);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::FolderTypingPaused(
                std::path::PathBuf::from("/srv/site"),
            )),
        );

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::FolderAnswerLate(
                std::path::PathBuf::from("/srv/site"),
            )),
        );

        let vm = run_form(&state).expect("form");
        assert!(!vm.can_run);
        assert_eq!(
            vm.volumes[0].problem,
            Some(i18n::t(Key::ContainersRunErrSourceNotAnswering))
        );
        assert!(vm.volumes[0].is_source_invalid);
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunWaitingForFolder));
    }

    #[test]
    fn the_footer_says_it_waits_while_a_folder_has_no_answer() {
        let mut state = with_volume_row("/srv/site", false);

        let vm = run_form(&state).expect("form");
        assert!(!vm.can_run);
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunWaitingForFolder));

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::FolderChecked {
                path: std::path::PathBuf::from("/srv/site"),
                exists: true,
            }),
        );

        let vm = run_form(&state).expect("form");
        assert!(vm.can_run);
        assert_eq!(
            vm.footer_hint,
            "Starts in the background on Docker 29.9.1."
        );
    }

    #[test]
    fn the_footer_asks_to_fix_fields_when_a_silent_folder_is_not_the_only_problem()
     {
        let mut state = with_volume_row("/srv/site", false);
        for intent in [
            ContainersIntent::Run(RunIntent::FolderTypingPaused(
                std::path::PathBuf::from("/srv/site"),
            )),
            ContainersIntent::Run(RunIntent::FolderAnswerLate(
                std::path::PathBuf::from("/srv/site"),
            )),
        ] {
            let _ = reduce_containers(&mut state, intent);
        }

        edit(&mut state, RunEdit::AddPort);
        edit(&mut state, RunEdit::PortContainer(0, String::from("web")));

        let vm = run_form(&state).expect("form");
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunFixFields));
    }

    #[test]
    fn turning_the_relabel_switch_on_shows_in_the_row() {
        let mut state = with_volume_row("/srv/site", true);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Edited(RunEdit::VolumeRelabel(
                0, true,
            ))),
        );

        let vm = run_form(&state).expect("form");
        assert!(vm.volumes[0].is_relabeled);
        assert!(
            state.run().form().expect("form").request().volumes()[0]
                .is_relabeled()
        );
    }

    fn here() -> ContainersState {
        state_with(
            &[EngineKind::Docker],
            Some(Ok(Some(fixtures::image_defaults()))),
        )
    }

    fn edit(state: &mut ContainersState, edit: RunEdit) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Run(RunIntent::Edited(edit)),
        );
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
        assert_eq!(vm.run_label, i18n::t(Key::ContainersActionRun));
        assert_eq!(vm.network, "bridge");
        assert!(vm.networks.contains(&String::from("bridge")));
        assert_eq!(vm.restart.policy, RestartPolicy::No);
        assert_eq!(vm.restarts.len(), 4);
        assert!(vm.disabled_restarts.is_empty());
        assert_eq!(vm.restart_hint, i18n::t(Key::ContainersRestartHintNo));
        assert_eq!(vm.progress, None);
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
            Some(i18n::t(Key::ContainersRunImagePulledFirst))
        );
        assert_eq!(
            not_pulled.as_deref(),
            Some(i18n::t(Key::ContainersRunImageNotPulled))
        );
        let vm = run_form(&state).expect("form");
        assert_eq!(vm.command_hint, i18n::t(Key::ContainersRunCommandHint));
        assert_eq!(vm.ports_hint, None);
    }

    #[test]
    fn while_the_image_is_checked_run_waits() {
        let state = state_with(&[EngineKind::Docker], None);

        let vm = run_form(&state).expect("form");

        assert!(!vm.can_run);
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunCheckingImage));
        assert_eq!(
            vm.image_hint.as_deref(),
            Some(i18n::t(Key::ContainersRunCheckingImage))
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
        assert_eq!(vm.footer_hint, i18n::t(Key::ContainersRunFixFields));
        let port = &vm.ports[0];
        assert_eq!(port.problem, Some(i18n::t(Key::ContainersRunErrPort)));
        assert!(port.is_host_invalid && !port.is_container_invalid);
        let volume = &vm.volumes[0];
        assert_eq!(
            volume.problem,
            Some(i18n::t(Key::ContainersRunErrContainerPath))
        );
        assert!(!volume.is_source_invalid && volume.is_target_invalid);
    }

    #[test]
    fn an_empty_image_is_a_problem() {
        let mut state = here();
        edit(&mut state, RunEdit::Image(String::new()));

        let vm = run_form(&state).expect("form");

        assert_eq!(vm.image_error, Some(i18n::t(Key::ContainersRunErrImage)));
        assert_eq!(vm.image_hint, None);
    }

    #[test]
    fn auto_remove_disables_the_other_restart_choices_with_a_hint() {
        let mut state = here();
        edit(&mut state, RunEdit::Restart(RestartPolicy::UnlessStopped));
        let before = run_form(&state).map(|vm| vm.restart_hint);

        edit(&mut state, RunEdit::AutoRemove(true));

        let vm = run_form(&state).expect("form");
        assert_eq!(
            before,
            Some(i18n::t(Key::ContainersRestartHintUnlessStopped))
        );
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
        assert_eq!(
            vm.restart_hint,
            i18n::t(Key::ContainersRestartHintAutoRemove)
        );
    }

    #[test]
    fn a_pull_shows_its_progress_in_the_footer_and_locks_the_form() {
        let mut state = state_with(&[EngineKind::Docker], Some(Ok(None)));
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::Submitted),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Run(RunIntent::PullProgressed(
                fixtures::pull_progress(),
            )),
        );

        let vm = run_form(&state).expect("form");

        let progress = vm.progress.expect("a pull line");
        assert_eq!(progress.text, i18n::containers_pulling("nginx:1.27"));
        assert!(progress.amount.is_some());
        assert!(!vm.is_editable);
        assert!(!vm.can_run);
        assert_eq!(vm.run_label, i18n::t(Key::ContainersRunStarting));
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
        let state = ContainersState::default();

        assert!(run_form(&state).is_none());
        assert!(image_panel(&state).is_none());
    }

    #[test]
    fn an_open_form_has_no_view_models_while_the_ctx_has_no_engine() {
        let state = here();
        let no_engine = RunCtx {
            engines: Vec::new(),
            is_selinux_enabled: false,
            home_folder: None,
        };

        let form = RunFormViewModel::from_state(state.run(), &no_engine);
        let panel = RunPanelViewModel::from_state(state.run(), &no_engine);

        assert!(form.is_none());
        assert!(panel.is_none());
        assert!(run_form(&state).is_some());
        assert!(image_panel(&state).is_some());
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
                    i18n::t(Key::ContainersFieldListensOn),
                    String::from("443/tcp, 80/tcp")
                ),
                (
                    i18n::t(Key::ContainersFieldCommand),
                    String::from("nginx -g \"daemon off;\"")
                ),
                (
                    i18n::t(Key::ContainersFieldEntrypoint),
                    String::from("/docker-entrypoint.sh")
                ),
                (i18n::t(Key::ContainersFieldWorkdir), String::from("/")),
            ]
        );
        assert_eq!(panel.hint, i18n::t(Key::ContainersRunPanelHint));
        assert_eq!(panel.error, None);
    }

    #[test]
    fn the_panel_says_when_the_image_is_missing_or_unknown() {
        let missing = state_with(&[EngineKind::Docker], Some(Ok(None)));
        let failed = state_with(
            &[EngineKind::Docker],
            Some(Err(ContainersError::new("store is locked"))),
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
                i18n::t(Key::ContainersImageMissing).to_string(),
                StatusTone::Warn
            ))
        );
        assert!(missing.fields.is_empty());
        assert_eq!(
            failed.error,
            Some(i18n::containers_filled(
                Key::TplContainersImageLookupFailed,
                "error",
                "store is locked"
            ))
        );
        assert_eq!(failed.status, None);
        assert_eq!(
            checking.status,
            Some((
                i18n::t(Key::ContainersRunCheckingImage).to_string(),
                StatusTone::Neutral
            ))
        );
        assert_eq!(empty.title, i18n::t(Key::ContainersRunContainer));
        assert_eq!(empty.hint, i18n::t(Key::ContainersRunNoImage));
        assert_eq!(empty.status, None);
    }
}
