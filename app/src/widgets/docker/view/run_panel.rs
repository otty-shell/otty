//! The right panel while the Run container form is open: what the typed
//! image sets on its own (spec Decision 18).

use iced::widget::{column, container, scrollable, svg, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::{fonts, typography};

use super::super::event::DockerIntent;
use super::super::run_model::ImagePanelViewModel;
use crate::icons;
use crate::theme::ThemeProps;

/// Padding of the panel (`padding: 14px 16px`), as the inspector.
const PADDING: Padding = Padding {
    top: 14.0,
    right: 16.0,
    bottom: 14.0,
    left: 16.0,
};
/// Space between the header, the hint and the list (`gap: 14px`).
const SECTION_GAP: f32 = 14.0;

/// Props for the image panel.
pub(crate) struct RunPanelProps<'a> {
    pub(crate) vm: ImagePanelViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the image, whether it is on this computer, and its ports,
/// command, entrypoint and working directory.
pub(crate) fn view(props: RunPanelProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut header = InspectorHeader::new(vm.title, palette);
    if let Some((status, tone)) = vm.status {
        header = header.status(StatusBadge::new(status, tone, palette));
    }
    let style = typography::SMALL;
    let hint = text(vm.hint)
        .size(style.size())
        .line_height(iced::Pixels(style.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted());

    let mut panel = column![header, hint]
        .spacing(SECTION_GAP)
        .width(Length::Fill);
    if let Some(error) = vm.error {
        panel = panel.push(Notice::new(
            error,
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        ));
    }
    if !vm.fields.is_empty() {
        let mut list = KeyValueList::new(palette);
        for (label, value) in vm.fields {
            list = list.entry(label, value);
        }
        panel = panel.push(list);
    }

    container(scrollable(panel).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::theme::{AppTheme, BuiltinTheme};

    fn nginx() -> ImagePanelViewModel {
        ImagePanelViewModel {
            title: String::from("nginx:1.27"),
            status: Some((
                String::from("On this computer, 192 MB"),
                StatusTone::Neutral,
            )),
            fields: vec![
                (i18n::t(Key::DockerFieldListensOn), String::from("80/tcp")),
                (
                    i18n::t(Key::DockerFieldEntrypoint),
                    String::from("/docker-entrypoint.sh"),
                ),
            ],
            hint: i18n::t(Key::DockerRunPanelHint),
            error: None,
        }
    }

    #[test]
    fn the_panel_shows_the_image_and_what_it_sets_in_both_themes() {
        for builtin in [BuiltinTheme::OttyDark, BuiltinTheme::OttyLight] {
            let theme = AppTheme::new(builtin);
            let mut ui = simulator(view(RunPanelProps {
                vm: nginx(),
                theme: ThemeProps::new(&theme),
            }));

            for shown in [
                "nginx:1.27",
                "On this computer, 192 MB",
                i18n::t(Key::DockerRunPanelHint),
                i18n::t(Key::DockerFieldListensOn),
                "80/tcp",
                "/docker-entrypoint.sh",
            ] {
                assert!(ui.find(shown).is_ok(), "{shown} should be visible");
            }
        }
    }

    #[test]
    fn a_failed_lookup_shows_the_engines_message() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let vm = ImagePanelViewModel {
            status: None,
            fields: Vec::new(),
            error: Some(String::from("Could not read the image: boom")),
            ..nginx()
        };
        let mut ui = simulator(view(RunPanelProps {
            vm,
            theme: ThemeProps::new(&theme),
        }));

        assert!(ui.find("Could not read the image: boom").is_ok());
    }
}
