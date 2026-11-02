//! The Run container form in the main area (spec Decision 18): Image,
//! Command, Ports, Environment, Volumes and Options sections, the engine's
//! notice on top, and a footer with the pull line, Cancel and Run.

use iced::widget::{column, row, svg, text};
use iced::{Alignment, Element, Length};
use otty_docker::PortProtocol;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::form_page::FormPage;
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::key_value_field::KeyValueField;
use otty_ui_kit::components::mapping_field::{MappingField, MappingRow};
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::segmented::{segmented, segmented_with_disabled};
use otty_ui_kit::components::select::select_owned;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::components::token_field::TokenField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::DockerIntent;
use super::super::run_form::RunEdit;
use super::super::run_model::{
    PortRowViewModel, RunFormViewModel, VolumeRowViewModel,
};
use super::form_dialog::pull_line;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// The two port protocols, in the order the rows show them.
const PROTOCOLS: [PortProtocol; 2] = [PortProtocol::Tcp, PortProtocol::Udp];
/// Width of the protocol control column (`width: 108px`).
const PROTOCOL_WIDTH: f32 = 108.0;
/// Width of the read-only switch column (`width: 104px`).
const READ_ONLY_WIDTH: f32 = 120.0;
/// Label size of a field without a kit label (`font-size: 12.5px`).
const LABEL_SIZE: f32 = 12.5;

type RunElement<'a> = Element<'a, DockerIntent>;

/// Props for the Run container form.
pub(crate) struct RunFormProps<'a> {
    pub(crate) vm: RunFormViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the Run container form.
pub(crate) fn view(props: RunFormProps<'_>) -> RunElement<'_> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut page = FormPage::new(palette).section(heading(palette));
    if let Some(notice) = vm.notice {
        page = page.section(
            Notice::new(
                notice,
                svg::Handle::from_memory(icons::NOTICE_ERROR),
                palette,
            )
            .on_dismiss(
                DockerIntent::RunNoticeDismissed,
                i18n::t(Key::DockerDismiss),
            ),
        );
    }

    page.section(image_section(&vm, palette))
        .section(command_section(&vm, palette))
        .section(ports_section(&vm, palette))
        .section(env_section(&vm, palette))
        .section(volumes_section(&vm, palette))
        .section(options_section(&vm, palette))
        .footer_bar(footer(&vm, palette))
        .into()
}

/// "Run container" and the line under it.
fn heading<'a>(palette: &UiPalette) -> RunElement<'a> {
    column![
        text(i18n::t(Key::DockerRunContainer))
            .size(typography::TITLE.size())
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        text(i18n::t(Key::DockerRunSubtitle))
            .size(typography::SMALL.size())
            .font(fonts::UI)
            .color(palette.text_muted()),
    ]
    .spacing(spacing::SPACE_1)
    .into()
}

/// Engine (with more than one), Image, Name and the pull switch.
fn image_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let is_editable = vm.is_editable;

    let mut image =
        TextField::new(i18n::t(Key::DockerFieldImage), vm.image, palette)
            .placeholder("nginx:1.27");
    if let Some(error) = vm.image_error {
        image = image.error(error);
    } else if let Some(hint) = &vm.image_hint {
        image = image.hint(hint.clone());
    }
    let mut name =
        TextField::new(i18n::t(Key::DockerFieldName), vm.name, palette)
            .placeholder(i18n::t(Key::DockerRunNamePlaceholder))
            .optional(true);
    let mut pull = Switch::new(vm.pulls_missing, palette)
        .label(i18n::t(Key::DockerRunPullMissing));
    if is_editable {
        image = image.on_input(|value| edit(RunEdit::Image(value)));
        name = name.on_input(|value| edit(RunEdit::Name(value)));
        pull = pull.on_toggle(|is_on| edit(RunEdit::PullMissing(is_on)));
    }

    let mut section = FormSection::new(i18n::t(Key::DockerFieldImage), palette);
    if !vm.engines.is_empty() {
        let engines = select_owned(
            vm.engines.clone(),
            Some(vm.engine.clone()),
            |choice| DockerIntent::RunEngineChosen(choice.address),
            palette,
        );
        section = section.field(labelled(
            i18n::t(Key::DockerFieldEngine),
            engines,
            palette,
        ));
    }

    section
        .field(
            row![image, name]
                .spacing(spacing::SPACE_4)
                .width(Length::Fill),
        )
        .field(pull)
        .into()
}

/// The command words.
fn command_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let mut command =
        TokenField::new(i18n::t(Key::DockerFieldCommand), vm.command, palette)
            .placeholder(i18n::t(Key::DockerRunCommandPlaceholder))
            .optional(true)
            .hint(vm.command_hint.clone());
    if vm.is_editable {
        command = command
            .on_input(|value| edit(RunEdit::CommandInput(value)))
            .on_submit(edit(RunEdit::CommandSubmit))
            .on_backspace_empty(edit(RunEdit::CommandBackspace))
            .on_remove(|index| edit(RunEdit::CommandRemove(index)));
    }

    FormSection::new(i18n::t(Key::DockerFieldCommand), palette)
        .field(command)
        .into()
}

/// Host port : container port rows with their protocol.
fn ports_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let mut field = MappingField::new(
        i18n::t(Key::DockerRunHostPort),
        i18n::t(Key::DockerRunContainerPort),
        palette,
    )
    .separator(":")
    .control_width(PROTOCOL_WIDTH)
    .add_label(i18n::t(Key::DockerRunAddPort));
    for (index, port) in vm.ports.iter().enumerate() {
        field = field.row(port_row(index, port, vm.is_editable, palette));
    }
    if vm.is_editable {
        field = field.on_add(edit(RunEdit::AddPort));
    }
    if let Some(hint) = &vm.ports_hint {
        field = field.hint(hint.clone());
    }

    FormSection::new(i18n::t(Key::DockerColumnPorts), palette)
        .field(field)
        .into()
}

/// One port row: inputs, the protocol and the cross.
fn port_row<'a>(
    index: usize,
    port: &PortRowViewModel<'a>,
    is_editable: bool,
    palette: &UiPalette,
) -> MappingRow<'a, DockerIntent> {
    let protocol = if is_editable {
        segmented(
            &PROTOCOLS,
            &port.protocol,
            move |protocol| edit(RunEdit::PortProtocol(index, protocol)),
            palette,
        )
    } else {
        segmented_with_disabled(
            &PROTOCOLS,
            &PROTOCOLS,
            &port.protocol,
            move |protocol| edit(RunEdit::PortProtocol(index, protocol)),
            palette,
        )
    };

    let mut row = MappingRow::new(port.host, port.container)
        .placeholders("8080", "80")
        .invalid(port.is_host_invalid, port.is_container_invalid)
        .control(protocol);
    if let Some(problem) = port.problem {
        row = row.error(problem);
    }
    if is_editable {
        row = row
            .on_source(move |value| edit(RunEdit::PortHost(index, value)))
            .on_target(move |value| edit(RunEdit::PortContainer(index, value)))
            .on_remove(
                edit(RunEdit::RemovePort(index)),
                i18n::t(Key::DockerRunRemovePort),
            );
    }
    row
}

/// Name = value rows.
fn env_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let mut field = KeyValueField::new("", vm.env, palette)
        .placeholders(
            i18n::t(Key::DockerFieldName),
            i18n::t(Key::DockerRunEnvValue),
        )
        .add_label(i18n::t(Key::DockerRunAddVariable));
    if vm.is_editable {
        field = field
            .on_name(|index, value| edit(RunEdit::EnvName(index, value)))
            .on_value(|index, value| edit(RunEdit::EnvValue(index, value)))
            .on_remove(|index| edit(RunEdit::RemoveEnv(index)))
            .on_add(edit(RunEdit::AddEnv));
    }

    FormSection::new(i18n::t(Key::FieldEnvironment), palette)
        .field(field)
        .into()
}

/// Source → path in the container rows with the read-only switch.
fn volumes_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let mut field = MappingField::new(
        i18n::t(Key::DockerRunVolumeSource),
        i18n::t(Key::DockerRunVolumeTarget),
        palette,
    )
    .control_width(READ_ONLY_WIDTH)
    .add_label(i18n::t(Key::DockerRunAddVolume));
    for (index, volume) in vm.volumes.iter().enumerate() {
        field = field.row(volume_row(index, volume, vm.is_editable, palette));
    }
    if vm.is_editable {
        field = field.on_add(edit(RunEdit::AddVolume));
    }

    FormSection::new(i18n::t(Key::DockerVolumes), palette)
        .field(field)
        .into()
}

/// One volume row: inputs, the read-only switch and the cross.
fn volume_row<'a>(
    index: usize,
    volume: &VolumeRowViewModel<'a>,
    is_editable: bool,
    palette: &UiPalette,
) -> MappingRow<'a, DockerIntent> {
    let mut read_only = Switch::new(volume.is_read_only, palette)
        .label(i18n::t(Key::DockerRunReadOnly));
    if is_editable {
        read_only = read_only.on_toggle(move |is_on| {
            edit(RunEdit::VolumeReadOnly(index, is_on))
        });
    }

    let mut row = MappingRow::new(volume.source, volume.target)
        .placeholders("otty-data", "/data")
        .invalid(volume.is_source_invalid, volume.is_target_invalid)
        .control(read_only);
    if volume.offers_relabel {
        row = row.detail(relabel_switch(index, volume, is_editable, palette));
    }
    if let Some(problem) = volume.problem {
        row = row.error(problem);
    }
    if is_editable {
        row = row
            .on_source(move |value| edit(RunEdit::VolumeSource(index, value)))
            .on_target(move |value| edit(RunEdit::VolumeTarget(index, value)))
            .on_remove(
                edit(RunEdit::RemoveVolume(index)),
                i18n::t(Key::DockerRunRemoveVolume),
            );
    }
    row
}

/// The SELinux switch of a folder row with its hint, shown under the row
/// on a computer with SELinux.
fn relabel_switch<'a>(
    index: usize,
    volume: &VolumeRowViewModel<'a>,
    is_editable: bool,
    palette: &UiPalette,
) -> RunElement<'a> {
    let mut switch = Switch::new(volume.is_relabeled, palette)
        .label(i18n::t(Key::DockerRunRelabel));
    if is_editable {
        switch = switch
            .on_toggle(move |is_on| edit(RunEdit::VolumeRelabel(index, is_on)));
    }
    let hint = text(i18n::t(Key::DockerRunRelabelHint))
        .size(typography::SMALL.size())
        .font(fonts::UI)
        .color(palette.text_muted());

    row![switch, hint]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// Network, Restart and the auto remove switch.
fn options_section<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let networks: Element<'a, DockerIntent> = select_owned(
        vm.networks.clone(),
        (!vm.network.is_empty()).then(|| vm.network.clone()),
        |network| edit(RunEdit::Network(network)),
        palette,
    );
    let disabled = if vm.is_editable {
        vm.disabled_restarts.clone()
    } else {
        vm.restarts.clone()
    };
    let restart = segmented_with_disabled(
        &vm.restarts,
        &disabled,
        &vm.restart,
        |choice| edit(RunEdit::Restart(choice.policy)),
        palette,
    );
    let restart = column![
        restart,
        text(vm.restart_hint)
            .size(typography::SMALL.size())
            .font(fonts::UI)
            .color(palette.text_muted()),
    ]
    .spacing(spacing::SPACE_2);
    let mut auto_remove = Switch::new(vm.is_auto_remove, palette)
        .label(i18n::t(Key::DockerRunAutoRemove));
    if vm.is_editable {
        auto_remove =
            auto_remove.on_toggle(|is_on| edit(RunEdit::AutoRemove(is_on)));
    }

    FormSection::new(i18n::t(Key::DockerRunSectionOptions), palette)
        .field(labelled(
            i18n::t(Key::DockerFieldNetwork),
            networks,
            palette,
        ))
        .field(labelled(i18n::t(Key::DockerFieldRestart), restart, palette))
        .field(auto_remove)
        .into()
}

/// The pull line, the hint, Cancel and Run.
fn footer<'a>(
    vm: &RunFormViewModel<'a>,
    palette: &UiPalette,
) -> RunElement<'a> {
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(DockerIntent::RunCancelled);
    let mut run = Button::new(vm.run_label, palette).kind(ButtonKind::Primary);
    if vm.can_run {
        run = run.on_press(DockerIntent::RunSubmitted);
    }
    let hint = text(vm.footer_hint.clone())
        .size(typography::SMALL.size())
        .font(fonts::UI)
        .color(palette.text_muted())
        .width(Length::Fill);

    let mut line = row![].spacing(spacing::SPACE_3).align_y(Alignment::Center);
    if let Some(progress) = vm.progress.clone() {
        line = line.push(pull_line(progress, palette));
    }

    line.push(hint).push(cancel).push(run).into()
}

/// A control under a label, for the kit parts without a label of their own.
fn labelled<'a>(
    label: &'static str,
    control: impl Into<RunElement<'a>>,
    palette: &UiPalette,
) -> RunElement<'a> {
    column![
        text(label)
            .size(LABEL_SIZE)
            .font(fonts::UI)
            .color(palette.text_body()),
        control.into(),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

/// Wrap a form edit.
fn edit(edit: RunEdit) -> DockerIntent {
    DockerIntent::RunEdited(edit)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::i18n::{self, Key};
    use crate::theme::{AppTheme, BuiltinTheme};
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::run_form::RunEdit;
    use crate::widgets::docker::run_model::run_form;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    /// Docker mode with the Run form open, `nginx:1.27` typed and the
    /// engine's answer `found`, then `edits`.
    fn state(
        found: Option<otty_docker::ImageDefaults>,
        edits: Vec<RunEdit>,
    ) -> DockerState {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
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
        let _ = reduce(
            &mut state,
            DockerIntent::ImageDefaultsLoaded {
                engine: docker.address().clone(),
                reference: String::from("nginx:1.27"),
                result: Ok(found),
            },
        );
        for edit in edits {
            let _ = reduce(&mut state, DockerIntent::RunEdited(edit));
        }
        state
    }

    fn ready() -> DockerState {
        state(Some(fixtures::image_defaults()), Vec::new())
    }

    /// A window tall enough for the whole form, so every part can be
    /// pressed without scrolling.
    fn tall<'a>(
        element: impl Into<Element<'a, DockerIntent>>,
    ) -> iced_test::Simulator<'a, DockerIntent> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 2400.0),
            element,
        )
    }

    /// Press the text `label` in the form drawn from `state`.
    fn press(state: &DockerState, label: &str) -> Vec<DockerIntent> {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let vm = run_form(state).expect("form");
        let mut ui = tall(view(RunFormProps {
            vm,
            theme: ThemeProps::new(&theme),
        }));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn the_form_shows_its_sections_in_both_themes() {
        let state = ready();
        for builtin in [BuiltinTheme::OttyDark, BuiltinTheme::OttyLight] {
            let theme = AppTheme::new(builtin);
            let vm = run_form(&state).expect("form");
            let mut ui = simulator(view(RunFormProps {
                vm,
                theme: ThemeProps::new(&theme),
            }));

            for shown in [
                i18n::t(Key::DockerRunContainer),
                i18n::t(Key::DockerRunSubtitle),
                i18n::t(Key::DockerFieldImage),
                i18n::t(Key::DockerRunPullMissing),
                i18n::t(Key::DockerColumnPorts),
                i18n::t(Key::DockerRunAddPort),
                i18n::t(Key::DockerRunAddVolume),
                i18n::t(Key::DockerRunSectionOptions),
                i18n::t(Key::DockerRestartUnlessStopped),
                i18n::t(Key::DockerRunAutoRemove),
                "On this computer, 192 MB",
                "Starts in the background on Docker 29.9.1.",
            ] {
                assert!(ui.find(shown).is_ok(), "{shown} should be visible");
            }
        }
    }

    #[test]
    fn run_sends_its_intent_when_the_form_is_ready() {
        let messages = press(&ready(), i18n::t(Key::DockerActionRun));

        assert_eq!(messages, vec![DockerIntent::RunSubmitted]);
    }

    #[test]
    fn run_is_disabled_while_a_field_is_wrong() {
        let state = state(
            Some(fixtures::image_defaults()),
            vec![
                RunEdit::AddPort,
                RunEdit::PortHost(0, String::from("80800")),
                RunEdit::PortContainer(0, String::from("443")),
            ],
        );

        let messages = press(&state, i18n::t(Key::DockerActionRun));

        assert!(messages.is_empty(), "{messages:?}");
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = simulator(view(RunFormProps {
            vm: run_form(&state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));
        assert!(ui.find(i18n::t(Key::DockerRunErrPort)).is_ok());
        assert!(ui.find(i18n::t(Key::DockerRunFixFields)).is_ok());
    }

    #[test]
    fn cancel_sends_its_intent() {
        let messages = press(&ready(), i18n::t(Key::ButtonCancel));

        assert_eq!(messages, vec![DockerIntent::RunCancelled]);
    }

    #[test]
    fn a_port_row_separates_host_and_container_with_a_colon() {
        let state =
            state(Some(fixtures::image_defaults()), vec![RunEdit::AddPort]);
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let vm = run_form(&state).expect("form");
        let mut ui = tall(view(RunFormProps {
            vm,
            theme: ThemeProps::new(&theme),
        }));

        assert!(ui.find(":").is_ok(), "the port separator should be a colon");
        assert!(ui.find("→").is_err(), "no arrow without volume rows");
    }

    /// The ready form with one volume row from `source`, SELinux on or off.
    fn with_volume_row(source: &str, is_selinux_enabled: bool) -> DockerState {
        let mut state = state(
            Some(fixtures::image_defaults()),
            vec![
                RunEdit::AddVolume,
                RunEdit::VolumeSource(0, source.to_string()),
                RunEdit::VolumeTarget(0, String::from("/data")),
            ],
        );
        state
            .run_form_mut()
            .expect("the form is open")
            .set_selinux(is_selinux_enabled);

        state
    }

    fn shows_relabel(state: &DockerState) -> bool {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = tall(view(RunFormProps {
            vm: run_form(state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));

        let has_switch = ui.find(i18n::t(Key::DockerRunRelabel)).is_ok();
        let has_hint = ui.find(i18n::t(Key::DockerRunRelabelHint)).is_ok();
        assert_eq!(has_switch, has_hint, "the switch and its hint go together");

        has_switch
    }

    #[test]
    fn a_folder_row_shows_the_relabel_switch_only_with_selinux() {
        assert!(shows_relabel(&with_volume_row("/srv/site", true)));
        assert!(!shows_relabel(&with_volume_row("/srv/site", false)));
    }

    #[test]
    fn a_named_volume_row_never_shows_the_relabel_switch() {
        assert!(!shows_relabel(&with_volume_row("otty-data", true)));
        assert!(!shows_relabel(&ready()));
    }

    #[test]
    fn the_relabel_switch_sends_its_new_state() {
        let state = with_volume_row("/srv/site", true);

        let messages = press(&state, i18n::t(Key::DockerRunRelabel));

        assert_eq!(
            messages,
            vec![DockerIntent::RunEdited(RunEdit::VolumeRelabel(0, true))]
        );
    }

    #[test]
    fn restart_starts_at_no() {
        let state = ready();

        let vm = run_form(&state).expect("form");

        assert_eq!(vm.restart.policy, otty_docker::RestartPolicy::No);
        assert_eq!(vm.restart_hint, i18n::t(Key::DockerRestartHintNo));
    }

    #[test]
    fn the_add_buttons_add_rows() {
        let state = ready();

        let port = press(&state, i18n::t(Key::DockerRunAddPort));
        let volume = press(&state, i18n::t(Key::DockerRunAddVolume));

        assert_eq!(port, vec![DockerIntent::RunEdited(RunEdit::AddPort)]);
        assert_eq!(volume, vec![DockerIntent::RunEdited(RunEdit::AddVolume)]);
    }

    #[test]
    fn the_auto_remove_switch_sends_its_new_state() {
        let messages = press(&ready(), i18n::t(Key::DockerRunAutoRemove));

        assert_eq!(
            messages,
            vec![DockerIntent::RunEdited(RunEdit::AutoRemove(true))]
        );
    }

    #[test]
    fn with_auto_remove_the_other_restart_choices_send_nothing() {
        let state = state(
            Some(fixtures::image_defaults()),
            vec![RunEdit::AutoRemove(true)],
        );

        let always = press(&state, i18n::t(Key::DockerRestartAlways));
        let no = press(&state, i18n::t(Key::DockerRestartNo));

        assert!(always.is_empty(), "{always:?}");
        assert_eq!(
            no,
            vec![DockerIntent::RunEdited(RunEdit::Restart(
                otty_docker::RestartPolicy::No
            ))]
        );
    }

    #[test]
    fn a_pull_shows_its_line_in_the_footer_and_run_waits() {
        let mut state = state(None, Vec::new());
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = simulator(view(RunFormProps {
            vm: run_form(&state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));

        assert!(ui.find(i18n::docker_pulling("nginx:1.27").as_str()).is_ok());
        ui.click(i18n::t(Key::DockerRunStarting))
            .expect("run button");
        assert!(ui.into_messages().next().is_none());
    }

    #[test]
    fn an_engine_error_shows_on_top_and_can_be_dismissed() {
        let mut state = ready();
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);
        let _ = reduce(
            &mut state,
            DockerIntent::RunFinished {
                engine: fixtures::engine(EngineKind::Docker).address().clone(),
                result: Err(String::from("port is already allocated")),
            },
        );
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = simulator(view(RunFormProps {
            vm: run_form(&state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));

        let notice = i18n::docker_action_failed(
            Key::TplDockerRunFailed,
            "nginx:1.27",
            "port is already allocated",
        );
        assert!(ui.find(notice.as_str()).is_ok());
    }

    #[test]
    fn typing_the_image_sends_the_edit() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let mut ui = simulator(view(RunFormProps {
            vm: run_form(&state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));

        ui.click("nginx:1.27").expect("image input");
        let _ = ui.typewrite("x");

        let messages: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(
            matches!(
                messages.as_slice(),
                [DockerIntent::RunEdited(RunEdit::Image(_))]
            ),
            "{messages:?}"
        );
    }
}
