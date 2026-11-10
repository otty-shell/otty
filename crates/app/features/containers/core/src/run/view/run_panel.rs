//! The right panel while the Run container form is open: what the typed
//! image sets on its own (spec Decision 18).

use iced::widget::{column, container, scrollable, svg, text};
use iced::{Element, Length, Padding};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::{fonts, typography};

use super::super::event::RunIntent;
use super::super::model::RunPanelViewModel;

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
    pub(crate) vm: RunPanelViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the image, whether it is on this computer, and its ports,
/// command, entrypoint and working directory.
pub(crate) fn view(props: RunPanelProps<'_>) -> Element<'_, RunIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut header = InspectorHeader::new(vm.title, palette);
    if let Some((status, tone)) = vm.status {
        // As in the inspector: a long status goes on to the next line, so the
        // whole text can be read at any panel width.
        header = header
            .status(StatusBadge::new(status, tone, palette).wrapping(true));
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

    // The padding is inside the scroller, so the thin bar runs along the edge
    // of the panel and never covers the text.
    scrollable(container(panel).padding(PADDING))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette))
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_app_i18n::{self as i18n, Key, Locale};
    use otty_app_theme::theme::{AppTheme, BuiltinTheme};
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;

    /// Width of a side panel at its minimum.
    const SIDE_PANEL_MINIMUM: f32 = 128.0;

    fn nginx() -> RunPanelViewModel {
        RunPanelViewModel {
            title: String::from("nginx:1.27"),
            status: Some((
                String::from("On this computer, 192 MB"),
                StatusTone::Neutral,
            )),
            fields: vec![
                (
                    i18n::t(Key::ContainersFieldListensOn),
                    String::from("80/tcp"),
                ),
                (
                    i18n::t(Key::ContainersFieldEntrypoint),
                    String::from("/docker-entrypoint.sh"),
                ),
            ],
            hint: i18n::t(Key::ContainersRunPanelHint),
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
                i18n::t(Key::ContainersRunPanelHint),
                i18n::t(Key::ContainersFieldListensOn),
                "80/tcp",
                "/docker-entrypoint.sh",
            ] {
                assert!(ui.find(shown).is_ok(), "{shown} should be visible");
            }
        }
    }

    /// The bounds of the status of the panel at `width`, with the room there
    /// is for it.
    fn status_at(status: &str, width: f32) -> (iced::Rectangle, f32) {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let vm = RunPanelViewModel {
            status: Some((status.to_string(), StatusTone::Neutral)),
            ..nginx()
        };
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (width, 800.0),
            view(RunPanelProps {
                vm,
                theme: ThemeProps::new(&theme),
            }),
        );

        let bounds = ui.find(status).expect("the status should show").bounds();

        (bounds, width - PADDING.right - bounds.x)
    }

    #[test]
    fn the_status_stays_whole_at_the_smallest_panel_in_every_locale() {
        for locale in Locale::ALL {
            let here = i18n::text_in(locale, Key::TplContainersImageHere)
                .replace("{size}", "192 MB");
            for status in [
                here.as_str(),
                i18n::text_in(locale, Key::ContainersImageMissing),
                i18n::text_in(locale, Key::ContainersRunCheckingImage),
            ] {
                let (one_line, _) = status_at(status, 800.0);
                let (narrow, room) = status_at(status, SIDE_PANEL_MINIMUM);

                let lines = (one_line.width / room).ceil();
                assert!(
                    narrow.width <= room + 0.5,
                    "{locale:?}: {status:?} is {narrow:?} in {room}"
                );
                assert!(
                    narrow.height + 0.5 >= one_line.height * lines,
                    "{locale:?}: {status:?} should take {lines} lines, is \
                     {narrow:?}"
                );
            }
        }
    }

    /// A low panel overflows, and its bar must not cover what it says: the
    /// scroller reaches the edge of the panel and the text keeps its padding.
    #[test]
    fn in_a_low_panel_the_bar_stays_off_the_text() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let width = SIDE_PANEL_MINIMUM;
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (width, 100.0),
            view(RunPanelProps {
                vm: nginx(),
                theme: ThemeProps::new(&theme),
            }),
        );

        let scroller = ui
            .find(|candidate: Candidate<'_>| match candidate {
                Candidate::Scrollable { bounds, .. } => Some(bounds),
                _ => None,
            })
            .expect("the panel should scroll");
        let title = ui.find("nginx:1.27").expect("the title").bounds();

        assert_eq!(scroller.x + scroller.width, width);
        assert!(title.x + title.width <= width - PADDING.right + 0.5);
    }

    #[test]
    fn a_status_that_fits_stays_on_one_line_as_before() {
        let status = "On this computer, 192 MB";

        let (wide, room) = status_at(status, 400.0);

        assert!(wide.width < room);
        assert!(wide.height < 2.0 * 14.0, "{wide:?}");
    }

    #[test]
    fn a_failed_lookup_shows_the_engines_message() {
        let theme = AppTheme::new(BuiltinTheme::OttyDark);
        let vm = RunPanelViewModel {
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
