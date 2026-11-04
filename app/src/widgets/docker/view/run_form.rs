//! The Run container form in the main area (spec Decision 18): Image,
//! Command, Ports, Environment, Volumes and Options sections, the engine's
//! notice on top, and a footer with the pull line or a hint, Cancel and Run.

use iced::widget::{column, container, row, svg, text};
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
/// Width of the read-only switch column: room for the switch with its label on
/// one line in the kit's fonts. The longest label is Portuguese, 139.4 px, then
/// Russian and French.
const READ_ONLY_WIDTH: f32 = 140.0;
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
        .footer(footer(&vm, palette))
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
            .optional(i18n::t(Key::FieldOptional));
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
            .optional(i18n::t(Key::FieldOptional))
            .remove_label(i18n::t(Key::ButtonRemove))
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
        .control(protocol)
        .remove_label(i18n::t(Key::DockerRunRemovePort));
    if let Some(problem) = port.problem {
        row = row.error(problem);
    }
    if is_editable {
        row = row
            .on_source(move |value| edit(RunEdit::PortHost(index, value)))
            .on_target(move |value| edit(RunEdit::PortContainer(index, value)))
            .on_remove(edit(RunEdit::RemovePort(index)));
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
        .add_label(i18n::t(Key::DockerRunAddVariable))
        .remove_label(i18n::t(Key::ButtonRemoveEnv));
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
        .control(read_only)
        .remove_label(i18n::t(Key::DockerRunRemoveVolume));
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
            .on_remove(edit(RunEdit::RemoveVolume(index)));
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

/// The pull line or the hint, Cancel and Run.
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

    // `Fill`, so that the row gives Cancel and Run their room first and the
    // text wraps in what is left.
    let status: RunElement<'a> = match vm.progress.clone() {
        Some(progress) => container(pull_line(progress, palette))
            .width(Length::Fill)
            .into(),
        None => text(vm.footer_hint.clone())
            .size(typography::SMALL.size())
            .font(fonts::UI)
            .color(palette.text_muted())
            .width(Length::Fill)
            .into(),
    };

    row![status, cancel, run]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
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
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::size;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::testing::alone;
    use crate::theme::{AppTheme, BuiltinTheme};
    use crate::widgets::docker::errors::DockerError;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::run_form::RunEdit;
    use crate::widgets::docker::run_model::run_form;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    /// The form in the area it has in the app.
    fn page(props: RunFormProps<'_>) -> RunElement<'_> {
        let palette = props.theme.theme.ui();

        ScreenArea::new(0, view(props), palette).into()
    }

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
        let mut ui = tall(page(RunFormProps {
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
            let mut ui = simulator(page(RunFormProps {
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
        let mut ui = simulator(page(RunFormProps {
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
        let mut ui = tall(page(RunFormProps {
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

    #[test]
    fn a_folder_that_does_not_answer_shows_the_hint_and_run_waits() {
        let mut state = with_volume_row("/srv/site", false);
        for intent in [
            DockerIntent::FolderTypingPaused(std::path::PathBuf::from(
                "/srv/site",
            )),
            DockerIntent::FolderAnswerLate(std::path::PathBuf::from(
                "/srv/site",
            )),
        ] {
            let _ = reduce(&mut state, intent);
        }

        let messages = press(&state, i18n::t(Key::DockerActionRun));

        assert!(messages.is_empty(), "{messages:?}");
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = tall(page(RunFormProps {
            vm: run_form(&state).expect("form"),
            theme: ThemeProps::new(&theme),
        }));
        assert!(
            ui.find(i18n::t(Key::DockerRunErrSourceNotAnswering))
                .is_ok()
        );
        assert!(ui.find(i18n::t(Key::DockerRunWaitingForFolder)).is_ok());
    }

    fn shows_relabel(state: &DockerState) -> bool {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = tall(page(RunFormProps {
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
        let mut ui = tall(page(RunFormProps {
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
                result: Err(DockerError::new("port is already allocated")),
            },
        );
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut ui = simulator(page(RunFormProps {
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

    /// Draws the form in a window of the given size.
    fn in_window<'a>(
        state: &'a DockerState,
        theme: &'a AppTheme,
        width: f32,
        height: f32,
    ) -> iced_test::Simulator<'a, DockerIntent> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            page(RunFormProps {
                vm: run_form(state).expect("form"),
                theme: ThemeProps::new(theme),
            }),
        )
    }

    /// Scrolls under `pointer` as far as `x` and `y` go.
    fn wheel(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
        pointer: iced::Point,
        x: f32,
        y: f32,
    ) {
        ui.point_at(pointer);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels { x, y },
            },
        )]);
    }

    /// How many scrollers the page has.
    fn scroller_count(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
    ) -> usize {
        let mut count = 0;

        let _ = ui.find(|candidate: iced_test::selector::Candidate<'_>| {
            if matches!(
                candidate,
                iced_test::selector::Candidate::Scrollable { .. }
            ) {
                count += 1;
            }
            None::<()>
        });

        count
    }

    #[test]
    fn in_a_small_window_the_page_has_one_scroller() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        assert_eq!(scroller_count(&mut ui), 1);
    }

    #[test]
    fn in_a_small_window_run_and_cancel_are_reached_by_scrolling_with_the_form()
    {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let run = i18n::t(Key::DockerActionRun);
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        let found = ui.find(run).expect("Run should exist");
        wheel(
            &mut ui,
            iced::Point::new(100.0, 100.0),
            -10_000.0,
            -10_000.0,
        );
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("Cancel should be in reach");
        ui.click(run).expect("Run should be in reach");
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        let edge = found.bounds().x + found.bounds().width;
        let form_right = size::SCREEN_MIN - spacing::SPACE_7;
        assert_eq!(found.visible_bounds(), None);
        assert!(
            edge <= form_right && edge > form_right - 20.0,
            "the buttons should keep the form's width: Run ends at {edge}"
        );
        assert_eq!(
            messages,
            vec![DockerIntent::RunCancelled, DockerIntent::RunSubmitted]
        );
    }

    #[test]
    fn in_a_narrow_tall_window_the_hint_and_the_fields_scroll_sideways_together()
     {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let hint = run_form(&state).expect("form").footer_hint;
        let title = i18n::t(Key::DockerRunContainer);
        let height = 2400.0;
        let mut ui = in_window(&state, &theme, 300.0, height);
        let start = ui.find(hint.as_str()).expect("the hint").bounds();
        let title_start = ui.find(title).expect("the title").bounds();

        wheel(&mut ui, iced::Point::new(100.0, 100.0), -20.0, 0.0);
        let shown = ui
            .find(hint.as_str())
            .expect("the hint should exist")
            .visible_bounds()
            .expect("the hint should be in view");
        let title_shown = ui
            .find(title)
            .expect("the title should exist")
            .visible_bounds()
            .expect("the title should be in view");

        assert_eq!(start.x, spacing::SPACE_7);
        assert_eq!(shown.x, start.x - 20.0);
        assert_eq!(title_shown.x, title_start.x - 20.0);
        assert!(
            shown.y > height - 60.0,
            "the bar is at the bottom: {shown:?}"
        );
    }

    /// Run takes the labels and hints of `locale` and is drawn at the form's
    /// smallest width, in a window with room for its height.
    fn assert_the_footer_fits(
        locale: i18n::Locale,
        hint: &str,
        run_label: &'static str,
    ) {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let mut vm = run_form(&state).expect("form");
        vm.footer_hint = hint.to_string();
        vm.run_label = run_label;
        let width = size::SCREEN_MIN;
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (width, 2400.0),
            page(RunFormProps {
                vm,
                theme: ThemeProps::new(&theme),
            }),
        );

        let hint = ui.find(hint).expect("the hint should exist");
        let cancel = ui.find(i18n::t(Key::ButtonCancel)).expect("Cancel");
        let run = ui.find(run_label).expect("Run").bounds();

        let (hint, cancel) = (hint.bounds(), cancel.bounds());
        assert_eq!(hint.x, spacing::SPACE_7, "{locale:?}");
        assert!(hint.x + hint.width <= cancel.x, "{locale:?}: {hint:?}");
        assert!(cancel.x + cancel.width <= run.x, "{locale:?}: {cancel:?}");
        assert!(run.x + run.width <= width - spacing::SPACE_7, "{locale:?}");
        assert!(
            (hint.center_y() - run.center_y()).abs() < 1.0,
            "{locale:?}: {hint:?} beside {run:?}"
        );
    }

    #[test]
    fn at_the_smallest_width_every_hint_stays_left_of_the_buttons_in_every_locale()
     {
        for locale in i18n::Locale::ALL {
            let run = i18n::text_in(locale, Key::DockerActionRun);
            let ready = i18n::text_in(locale, Key::TplDockerRunReady)
                .replace("{engine}", "Docker 29.9.1");

            assert_the_footer_fits(locale, &ready, run);
            for key in [
                Key::DockerRunFixFields,
                Key::DockerRunCheckingImage,
                Key::DockerRunWaitingForFolder,
            ] {
                assert_the_footer_fits(locale, i18n::text_in(locale, key), run);
            }
        }
    }

    #[test]
    fn at_the_smallest_width_a_long_pull_line_wraps_and_keeps_the_buttons() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut state = state(None, Vec::new());
        let long = "ghcr.io/some-organisation/a-rather-long-image-name:1.2.3";
        let _ = reduce(
            &mut state,
            DockerIntent::RunEdited(RunEdit::Image(long.to_string())),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ImageDefaultsLoaded {
                engine: fixtures::engine(EngineKind::Docker).address().clone(),
                reference: long.to_string(),
                result: Ok(None),
            },
        );
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );
        let width = size::SCREEN_MIN;
        let mut ui = in_window(&state, &theme, width, 2400.0);

        let line = i18n::docker_pulling(long);
        let line = ui.find(line.as_str()).expect("the pull line").bounds();
        let cancel = ui.find(i18n::t(Key::ButtonCancel)).expect("Cancel");
        let run = ui.find(i18n::t(Key::DockerRunStarting)).expect("Run");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("Cancel should be in reach");
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        let (cancel, run) = (cancel.bounds(), run.bounds());
        assert!(line.x + line.width <= cancel.x, "{line:?} and {cancel:?}");
        assert!(cancel.width > 30.0, "Cancel keeps its size: {cancel:?}");
        assert!(run.width > 30.0, "Run keeps its size: {run:?}");
        assert!(run.x + run.width <= width - spacing::SPACE_7);
        assert_eq!(messages, vec![DockerIntent::RunCancelled]);
    }

    #[test]
    fn at_a_normal_width_the_pull_line_stays_on_one_line() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut state = state(None, Vec::new());
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );
        let progress = run_form(&state)
            .expect("form")
            .progress
            .expect("a pull is running");
        let amount = progress.amount.expect("the pull has an amount");
        let mut ui = in_window(&state, &theme, 1024.0, 2400.0);

        let line = ui.find(progress.text.as_str()).expect("the line").bounds();
        let amount = ui.find(amount.as_str()).expect("the amount").bounds();

        assert_eq!(line.x, (1024.0 - size::FORM) / 2.0);
        assert!((line.center_y() - amount.center_y()).abs() < 1.0);
    }

    #[test]
    fn in_a_low_window_run_comes_after_the_fields_and_is_reached_by_scrolling()
    {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let run = i18n::t(Key::DockerActionRun);
        let mut ui = in_window(&state, &theme, 1024.0, 400.0);

        let hidden = ui.find(run).expect("Run should exist");
        let last = ui
            .find(i18n::t(Key::DockerRunAutoRemove))
            .expect("the last field should exist")
            .bounds();
        wheel(&mut ui, iced::Point::new(100.0, 100.0), 0.0, -10_000.0);
        ui.click(run).expect("Run should be in reach");
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        assert_eq!(hidden.visible_bounds(), None);
        assert!(hidden.bounds().y > last.y + last.height);
        assert_eq!(messages, vec![DockerIntent::RunSubmitted]);
    }

    #[test]
    fn in_a_small_window_the_far_controls_are_reached_by_scrolling() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let always = i18n::t(Key::DockerRestartAlways);
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        let hidden = ui.find(always).expect("the choice should exist");
        wheel(
            &mut ui,
            iced::Point::new(100.0, 100.0),
            -10_000.0,
            -10_000.0,
        );
        ui.click(always).expect("the choice should be in reach");
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        assert_eq!(hidden.visible_bounds(), None);
        assert_eq!(
            messages,
            vec![edit(RunEdit::Restart(otty_docker::RestartPolicy::Always))]
        );
    }

    #[test]
    fn in_a_small_window_typing_still_reaches_the_image() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        wheel(&mut ui, iced::Point::new(100.0, 100.0), -20.0, -20.0);
        ui.click("nginx:1.27")
            .expect("the image should be in reach");
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

    #[test]
    fn in_a_tall_wide_window_the_bar_is_at_the_bottom_and_lines_up_with_the_fields()
     {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let (width, height) = (1400.0, 2400.0);
        let run = i18n::t(Key::DockerActionRun);
        let hint = run_form(&state).expect("form").footer_hint;
        let mut ui = in_window(&state, &theme, width, height);

        let edge = ui.find(run).expect("Run should exist").bounds();
        let title = ui
            .find(i18n::t(Key::DockerRunContainer))
            .expect("the title should exist")
            .bounds();
        let shown = ui
            .find(hint.as_str())
            .expect("the hint should exist")
            .visible_bounds()
            .expect("the hint should be in view");

        let form_right = (width + size::FORM) / 2.0;
        let right = edge.x + edge.width;
        assert!(right <= form_right && right > form_right - 20.0, "{right}");
        assert!(edge.y > height - 60.0, "{edge:?}");
        assert_eq!(shown.x, title.x);
        assert_eq!(shown.x, (width - size::FORM) / 2.0);
    }

    /// The smallest width the source and target inputs of a volume row may
    /// have: about 16 characters of the mono font inside their padding.
    const VOLUME_INPUT_MIN: f32 = 150.0;

    #[test]
    fn at_the_smallest_width_the_inputs_of_a_volume_row_stay_usable_and_the_row_fits()
     {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = with_volume_row("otty-data", false);
        let width = size::SCREEN_MIN;
        let mut ui = in_window(&state, &theme, width, 2400.0);

        let source = ui
            .find(i18n::t(Key::DockerRunVolumeSource))
            .expect("the source heading")
            .bounds();
        let target = ui
            .find(i18n::t(Key::DockerRunVolumeTarget))
            .expect("the target heading")
            .bounds();

        let gap = spacing::SPACE_2;
        let row_right = target.x
            + target.width
            + gap
            + READ_ONLY_WIDTH
            + gap
            + size::CONTROL_MD;
        assert_eq!(source.width, target.width);
        assert!(source.width >= VOLUME_INPUT_MIN, "{source:?}");
        assert_eq!(row_right, width - spacing::SPACE_7);
    }

    /// Says whether Korean can be measured here. A computer whose only Hangul
    /// is a fallback font with next to no syllables breaks them into letters
    /// three times as wide, and those widths say nothing about the form. With
    /// no Hangul at all each syllable is one box, which fits like a syllable.
    fn hangul_stays_whole() -> bool {
        const SIZE: f32 = 16.0;

        let syllable = "읽";
        let shown: RunElement<'_> = text(syllable)
            .font(fonts::UI)
            .size(SIZE)
            .shaping(iced::widget::text::Shaping::Advanced)
            .into();
        let width = simulator(shown)
            .find(syllable)
            .expect("the syllable")
            .bounds()
            .width;

        width < 1.5 * SIZE
    }

    /// Takes Korean out of a test on a computer that breaks up its syllables.
    /// Tests in `crate::fonts` check that this does not happen with a font
    /// that has them.
    fn can_measure(locale: i18n::Locale) -> bool {
        if locale != i18n::Locale::Ko || hangul_stays_whole() {
            return true;
        }

        eprintln!("skipped Korean: this computer breaks Hangul into letters");

        false
    }

    #[test]
    fn the_read_only_label_reads_whole_in_every_locale() {
        if !alone::is_alone("the_read_only_label_reads_whole_in_every_locale") {
            return;
        }
        alone::load_kit_fonts();

        let palette = UiPalette::otty_dark();
        for locale in i18n::Locale::ALL {
            if !can_measure(locale) {
                continue;
            }
            let label = i18n::text_in(locale, Key::DockerRunReadOnly);
            let needed = Switch::<DockerIntent>::new(false, &palette)
                .label(label)
                .width();

            assert!(
                needed <= READ_ONLY_WIDTH,
                "{locale:?}: {label} needs {needed} in {READ_ONLY_WIDTH}"
            );
        }
    }

    /// Text size of the kit's token input.
    const TOKEN_INPUT_TEXT_SIZE: f32 = 12.5;
    /// Room left of the text of the kit's token input.
    const TOKEN_INPUT_PADDING_LEFT: f32 = 5.0;

    #[test]
    fn the_command_placeholder_reads_whole_at_the_smallest_width_in_every_locale()
     {
        if !alone::is_alone(
            "the_command_placeholder_reads_whole_at_the_smallest_width",
        ) {
            return;
        }
        alone::load_kit_fonts();

        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        for locale in i18n::Locale::ALL {
            if !can_measure(locale) {
                continue;
            }
            i18n::set_locale(locale);
            let placeholder = i18n::t(Key::DockerRunCommandPlaceholder);
            let shown: RunElement<'_> = text(placeholder)
                .font(fonts::MONO)
                .size(TOKEN_INPUT_TEXT_SIZE)
                .shaping(iced::widget::text::Shaping::Advanced)
                .into();
            let needed =
                simulator(shown).find(placeholder).expect("text").bounds();
            let state = state(
                Some(fixtures::image_defaults()),
                vec![RunEdit::CommandInput(String::from("typed"))],
            );
            let width = size::SCREEN_MIN;
            let mut ui = in_window(&state, &theme, width, 2400.0);

            let input = ui.find("typed").expect("the input").bounds();
            let title = ui
                .find(i18n::t(Key::DockerRunContainer))
                .expect("the title")
                .bounds();

            assert!(
                input.width - TOKEN_INPUT_PADDING_LEFT >= needed.width,
                "{locale:?}: {placeholder} needs {} in {}",
                needed.width,
                input.width
            );
            assert!(
                input.x - title.x < spacing::SPACE_3,
                "{locale:?}: the input should start its line: {input:?}"
            );
            assert!(
                input.x + input.width <= width - spacing::SPACE_7,
                "{locale:?}: the input should stay in the form: {input:?}"
            );
        }
    }

    #[test]
    fn the_optional_marker_is_in_the_language_of_the_form() {
        if !alone::is_alone(
            "the_optional_marker_is_in_the_language_of_the_form",
        ) {
            return;
        }

        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        for locale in i18n::Locale::ALL {
            i18n::set_locale(locale);
            let marker = i18n::t(Key::FieldOptional);
            let state = ready();
            let mut ui = in_window(&state, &theme, 1024.0, 2400.0);

            assert!(ui.find(marker).is_ok(), "{locale:?}: {marker}");
            assert_eq!(
                ui.find("optional").is_ok(),
                marker == "optional",
                "{locale:?}"
            );
        }
    }

    /// Presses the select under the label `label`, then the place of its second
    /// item, as `Settings` does for its list: the items are not text a test can
    /// find. Pressing an item works only if the list opened under its field.
    fn press_second_item_of(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
        label: &str,
    ) {
        let label = ui
            .find(label)
            .expect("the label should exist")
            .visible_bounds()
            .expect("the label should be in view");
        let x = label.x + 40.0;
        let field_y = label.y + label.height + size::CONTROL_LG / 2.0;

        ui.point_at(iced::Point::new(x, field_y));
        let _ = ui.simulate(iced_test::simulator::click());
        ui.point_at(iced::Point::new(x, field_y + 2.0 * size::CONTROL_LG));
        let _ = ui.simulate(iced_test::simulator::click());
    }

    #[test]
    fn in_a_scrolled_small_window_the_network_list_opens_under_its_field_and_picks()
     {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let label = i18n::t(Key::DockerFieldNetwork);
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        let at_rest = ui.find(label).expect("the label").bounds();
        wheel(
            &mut ui,
            iced::Point::new(100.0, 100.0),
            -20.0,
            100.0 - at_rest.y,
        );
        press_second_item_of(&mut ui, label);
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        assert!(
            matches!(
                messages.as_slice(),
                [DockerIntent::RunEdited(RunEdit::Network(_))]
            ),
            "{messages:?}"
        );
    }

    #[test]
    fn in_a_scrolled_small_window_the_engine_list_opens_under_its_field_and_picks()
     {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let mut state = ready();
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let label = i18n::t(Key::DockerFieldEngine);
        let mut ui = in_window(&state, &theme, 300.0, 400.0);

        wheel(&mut ui, iced::Point::new(100.0, 100.0), -20.0, -80.0);
        press_second_item_of(&mut ui, label);
        let messages: Vec<DockerIntent> = ui.into_messages().collect();

        assert!(
            matches!(messages.as_slice(), [DockerIntent::RunEngineChosen(_)]),
            "{messages:?}"
        );
    }

    #[test]
    fn typing_the_image_sends_the_edit() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let state = ready();
        let mut ui = simulator(page(RunFormProps {
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
