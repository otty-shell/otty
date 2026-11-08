use iced::Element;
use iced::widget::{row, text};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::form_page::FormPage;
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::SettingsIntent;
use super::super::model::SettingsViewModel;
use super::super::types::LanguageSetting;
use crate::i18n::{self, Key};
use crate::theme::{BuiltinTheme, ThemeProps};

/// Props for the settings form view.
pub(crate) struct SettingsFormProps<'a> {
    pub(crate) vm: SettingsViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the settings page: Appearance, General and Terminal sections on
/// one page, with Reset and Save in the footer.
pub(crate) fn view(
    props: SettingsFormProps<'_>,
) -> Element<'_, SettingsIntent> {
    let palette = props.theme.theme.ui();

    FormPage::new(palette)
        .section(page_title(palette))
        .section(appearance_section(&props, palette))
        .section(general_section(&props, palette))
        .section(terminal_section(&props, palette))
        .footer(footer(&props, palette))
        .into()
}

fn page_title<'a>(palette: &UiPalette) -> Element<'a, SettingsIntent> {
    let style = typography::TITLE;

    text(i18n::t(Key::SettingsTabTitle))
        .size(style.size())
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .into()
}

/// Reset and Save, which work only while there is something to save.
fn footer<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent> {
    let is_enabled = props.vm.is_dirty && !props.vm.is_saving;

    let mut save = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary);
    let mut reset =
        Button::new(i18n::t(Key::ButtonReset), palette).kind(ButtonKind::Ghost);
    if is_enabled {
        save = save.on_press(SettingsIntent::Save);
        reset = reset.on_press(SettingsIntent::Reset);
    }

    row![reset, save].spacing(spacing::SPACE_3).into()
}

fn appearance_section<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent> {
    let theme_selector = segmented(
        &BuiltinTheme::ALL,
        &props.vm.draft.theme(),
        SettingsIntent::ThemeSelected,
        palette,
    );

    FormSection::new(i18n::t(Key::SectionAppearance), palette)
        .field(
            SettingRow::new(i18n::t(Key::FieldTheme), palette)
                .description(i18n::t(Key::DescTheme))
                .control(theme_selector),
        )
        .into()
}

fn general_section<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent> {
    let language_selector = select(
        LanguageSetting::all(),
        Some(props.vm.draft.language()),
        SettingsIntent::LanguageChanged,
        palette,
    );

    FormSection::new(i18n::t(Key::SectionGeneral), palette)
        .field(
            SettingRow::new(i18n::t(Key::FieldLanguage), palette)
                .description(i18n::t(Key::DescLanguage))
                .control(language_selector),
        )
        .into()
}

fn terminal_section<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent> {
    let is_editable = !props.vm.is_saving;

    // The label stays empty: the row around the field already names it.
    let mut shell =
        TextField::new("", props.vm.draft.terminal_shell(), palette);
    let mut editor =
        TextField::new("", props.vm.draft.terminal_editor(), palette);
    let mut equalize_panes =
        Switch::new(props.vm.draft.equalize_panes(), palette);
    if is_editable {
        shell = shell.on_input(SettingsIntent::ShellChanged);
        editor = editor.on_input(SettingsIntent::EditorChanged);
        equalize_panes =
            equalize_panes.on_toggle(SettingsIntent::EqualizePanesToggled);
    }

    FormSection::new(i18n::t(Key::SectionTerminal), palette)
        .field(
            SettingRow::new(i18n::t(Key::FieldShell), palette)
                .description(i18n::t(Key::DescShell))
                .control(shell),
        )
        .field(
            SettingRow::new(i18n::t(Key::FieldDefaultEditor), palette)
                .description(i18n::t(Key::DescDefaultEditor))
                .control(editor),
        )
        .field(
            SettingRow::new(i18n::t(Key::OptionEqualizePanes), palette)
                .description(i18n::t(Key::DescEqualizePanes))
                .control(equalize_panes),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Theme;
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::size;

    use super::*;
    use crate::testing::temp_dir::TempDir;
    use crate::theme::{AppTheme, BuiltinTheme};
    use crate::widgets::settings::types::SettingsData;

    /// Height of the kit's footer bar (`.form-foot`).
    const BAR_HEIGHT: f32 = 60.0;

    /// The form in the area it has in the app.
    fn page(props: SettingsFormProps<'_>) -> Element<'_, SettingsIntent> {
        let palette = props.theme.theme.ui();

        ScreenArea::new(0, view(props), palette).into()
    }

    fn click_in_form(
        draft: &SettingsData,
        is_dirty: bool,
        label: &str,
    ) -> Vec<SettingsIntent> {
        let app_theme = AppTheme::default();
        let props = SettingsFormProps {
            vm: SettingsViewModel {
                draft,
                is_dirty,
                is_saving: false,
            },
            theme: ThemeProps::new(&app_theme),
        };
        let mut ui = simulator(page(props));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
    }

    /// Draws the form with an unsaved change in a window of the given width.
    fn dirty_form_in_window<'a>(
        draft: &'a SettingsData,
        app_theme: &'a AppTheme,
        width: f32,
    ) -> Simulator<'a, SettingsIntent> {
        dirty_form_in_size(draft, app_theme, width, 600.0)
    }

    /// Draws the form with an unsaved change in a window of the given size.
    fn dirty_form_in_size<'a>(
        draft: &'a SettingsData,
        app_theme: &'a AppTheme,
        width: f32,
        height: f32,
    ) -> Simulator<'a, SettingsIntent> {
        let props = SettingsFormProps {
            vm: SettingsViewModel {
                draft,
                is_dirty: true,
                is_saving: false,
            },
            theme: ThemeProps::new(app_theme),
        };

        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            page(props),
        )
    }

    /// Right edge of the Save label in the layout, before any scrolling.
    fn save_right_edge(ui: &mut Simulator<'_, SettingsIntent>) -> f32 {
        let save = ui.find("Save").expect("Save should exist").bounds();

        save.x + save.width
    }

    /// Scrolls under the pointer as far as `x` and `y` go.
    fn wheel(ui: &mut Simulator<'_, SettingsIntent>, x: f32, y: f32) {
        ui.point_at(iced::Point::new(100.0, 100.0));
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels { x, y },
            },
        )]);
    }

    /// How many scrollers the page has.
    fn scroller_count(ui: &mut Simulator<'_, SettingsIntent>) -> usize {
        let mut count = 0;

        let _ = ui.find(|candidate: Candidate<'_>| {
            if matches!(candidate, Candidate::Scrollable { .. }) {
                count += 1;
            }
            None::<()>
        });

        count
    }

    #[test]
    fn given_a_small_window_then_the_page_has_one_scroller() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = dirty_form_in_size(&draft, &app_theme, 300.0, 300.0);

        assert_eq!(scroller_count(&mut ui), 1);
    }

    #[test]
    fn given_a_normal_window_then_reset_and_save_sit_at_the_bottom_right() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let (width, height) = (1024.0, 768.0);
        let mut ui = dirty_form_in_size(&draft, &app_theme, width, height);

        let save = ui.find("Save").expect("Save should exist");
        let reset = ui.find("Reset").expect("Reset should exist");
        let title = ui
            .find(i18n::t(Key::SettingsTabTitle))
            .expect("the title should exist");
        let last = ui
            .find(i18n::t(Key::DescEqualizePanes))
            .expect("the last field should exist");

        let (save, reset) = (save.bounds(), reset.bounds());
        assert!(save.y > last.bounds().y + last.bounds().height);
        assert!(save.y > height - BAR_HEIGHT && save.y < height);
        assert_eq!(reset.y, save.y);
        assert!(reset.x + reset.width < save.x, "Reset comes before Save");
        let form_right = (width + size::FORM) / 2.0;
        assert!(save.x + save.width <= form_right);
        assert!(save.x + save.width > form_right - 20.0);
        assert!(title.bounds().y < last.bounds().y);
    }

    /// Reset has the look of Cancel in the other forms, with Save beside it,
    /// enabled and disabled.
    #[test]
    fn given_any_state_and_theme_then_reset_looks_like_cancel() {
        let folder = TempDir::new("settings-reset");
        let draft = SettingsData::default();
        let window = (300.0, 60.0);

        for builtin in [BuiltinTheme::OttyDark, BuiltinTheme::OttyLight] {
            for is_dirty in [true, false] {
                let reference =
                    folder.path().join(format!("{builtin:?}-{is_dirty}"));
                let app_theme = AppTheme::new(builtin);
                let palette = app_theme.ui();
                let props = SettingsFormProps {
                    vm: SettingsViewModel {
                        draft: &draft,
                        is_dirty,
                        is_saving: false,
                    },
                    theme: ThemeProps::new(&app_theme),
                };
                let mut cancel_look =
                    Button::new(i18n::t(Key::ButtonReset), palette)
                        .kind(ButtonKind::Ghost);
                let mut save = Button::new(i18n::t(Key::ButtonSave), palette)
                    .kind(ButtonKind::Primary);
                if is_dirty {
                    cancel_look = cancel_look.on_press(SettingsIntent::Reset);
                    save = save.on_press(SettingsIntent::Save);
                }
                let expected: Element<'_, SettingsIntent> =
                    row![cancel_look, save].spacing(spacing::SPACE_3).into();

                let expected_saved = Simulator::with_size(
                    iced::Settings::default(),
                    window,
                    expected,
                )
                .snapshot(&Theme::Dark)
                .expect("the buttons should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");
                let footer_matches = Simulator::with_size(
                    iced::Settings::default(),
                    window,
                    footer(&props, palette),
                )
                .snapshot(&Theme::Dark)
                .expect("the footer should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");

                assert!(expected_saved);
                assert!(footer_matches, "{builtin:?}, dirty {is_dirty}");
            }
        }
    }

    #[test]
    fn given_a_low_window_then_save_comes_after_the_fields_and_is_reached_by_scrolling()
     {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = dirty_form_in_size(&draft, &app_theme, 1024.0, 200.0);

        let hidden = ui.find("Save").expect("Save should exist");
        wheel(&mut ui, 0.0, -10_000.0);
        ui.click("Save").expect("Save should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert_eq!(hidden.visible_bounds(), None);
        assert!(matches!(messages.as_slice(), [SettingsIntent::Save]));
    }

    #[test]
    fn given_a_clean_draft_when_scrolled_to_save_and_reset_then_they_stay_off()
    {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let props = SettingsFormProps {
            vm: SettingsViewModel {
                draft: &draft,
                is_dirty: false,
                is_saving: false,
            },
            theme: ThemeProps::new(&app_theme),
        };
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 200.0),
            page(props),
        );

        wheel(&mut ui, 0.0, -10_000.0);
        ui.click("Save").expect("Save should be in reach");
        ui.click("Reset").expect("Reset should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_a_saving_form_then_save_and_reset_send_nothing() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let props = SettingsFormProps {
            vm: SettingsViewModel {
                draft: &draft,
                is_dirty: true,
                is_saving: true,
            },
            theme: ThemeProps::new(&app_theme),
        };
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 768.0),
            page(props),
        );

        ui.click("Save").expect("Save should be in reach");
        ui.click("Reset").expect("Reset should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_a_normal_window_then_the_language_list_opens_and_picks() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let (width, height) = (1024.0, 600.0);
        let mut ui = dirty_form_in_size(&draft, &app_theme, width, height);

        // As in the narrow test below, the control is found by place.
        let name = ui.find(i18n::t(Key::FieldLanguage)).expect("row name");
        let note = ui.find(i18n::t(Key::DescLanguage)).expect("row note");
        let middle =
            (name.bounds().y + note.bounds().y + note.bounds().height) / 2.0;
        let x = (width + size::FORM) / 2.0 - 20.0;
        ui.point_at(iced::Point::new(x, middle));
        let _ = ui.simulate(iced_test::simulator::click());
        ui.point_at(iced::Point::new(x, middle + 2.0 * size::CONTROL_LG));
        let _ = ui.simulate(iced_test::simulator::click());
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(
            matches!(messages.as_slice(), [SettingsIntent::LanguageChanged(_)]),
            "{messages:?}"
        );
    }

    #[test]
    fn given_less_room_than_the_minimum_then_save_is_reached_by_scrolling() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = dirty_form_in_window(&draft, &app_theme, 300.0);

        let edge = save_right_edge(&mut ui);
        let save = ui.find("Save").expect("Save should exist");
        wheel(&mut ui, -10_000.0, -10_000.0);
        ui.click("Save").expect("Save should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        let form_right = size::SCREEN_MIN - spacing::SPACE_7;
        assert!(
            edge <= form_right && edge > form_right - 20.0,
            "the form should keep its minimum width: Save ends at {edge}"
        );
        assert_eq!(save.visible_bounds(), None);
        assert!(matches!(messages.as_slice(), [SettingsIntent::Save]));
    }

    #[test]
    fn given_less_room_than_the_minimum_when_scrolled_then_the_language_list_opens_and_picks()
     {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = dirty_form_in_window(&draft, &app_theme, 300.0);

        wheel(&mut ui, -10_000.0, 0.0);

        // The dropdown shows its text itself, so the test finds it by place:
        // the control sits at the right edge, level with the middle of its row.
        let name = ui.find(i18n::t(Key::FieldLanguage)).expect("row name");
        let note = ui.find(i18n::t(Key::DescLanguage)).expect("row note");
        let middle =
            (name.bounds().y + note.bounds().y + note.bounds().height) / 2.0;
        let x = 300.0 - spacing::SPACE_7 - 20.0;
        ui.point_at(iced::Point::new(x, middle));
        let _ = ui.simulate(iced_test::simulator::click());
        ui.point_at(iced::Point::new(x, middle + 2.0 * size::CONTROL_LG));
        let _ = ui.simulate(iced_test::simulator::click());
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(
            matches!(messages.as_slice(), [SettingsIntent::LanguageChanged(_)]),
            "{messages:?}"
        );
    }

    #[test]
    fn given_a_normal_window_then_the_form_is_as_wide_as_before() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = dirty_form_in_window(&draft, &app_theme, 1024.0);

        let last_choice = ui.find("OTTY Light").expect("the theme should show");

        let edge = last_choice.bounds().x + last_choice.bounds().width;
        let form_right = (1024.0 + size::FORM) / 2.0;
        assert!(edge <= form_right && edge > form_right - 30.0, "{edge}");
    }

    #[test]
    fn given_dirty_draft_when_save_clicked_then_save_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_form(&draft, true, "Save");

        assert!(matches!(messages.as_slice(), [SettingsIntent::Save]));
    }

    #[test]
    fn given_clean_draft_when_save_clicked_then_nothing_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_form(&draft, false, "Save");

        assert!(messages.is_empty());
    }

    #[test]
    fn given_dirty_draft_when_reset_clicked_then_reset_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_form(&draft, true, "Reset");

        assert!(matches!(messages.as_slice(), [SettingsIntent::Reset]));
    }

    #[test]
    fn given_light_theme_clicked_then_theme_selected_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_form(&draft, false, "OTTY Light");

        assert!(matches!(
            messages.as_slice(),
            [SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight)]
        ));
    }

    #[test]
    fn given_settings_page_then_all_sections_and_descriptions_are_visible() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let props = SettingsFormProps {
            vm: SettingsViewModel {
                draft: &draft,
                is_dirty: false,
                is_saving: false,
            },
            theme: ThemeProps::new(&app_theme),
        };
        let mut ui = simulator(page(props));

        for text in [
            "Appearance",
            "General",
            "Terminal",
            "The window and the terminal switch together",
            "Menus, forms and messages",
            "Program started in every new terminal",
            "Opens files from the Files panel",
            "Give every pane the same size after a split",
        ] {
            assert!(ui.find(text).is_ok(), "{text} should be visible");
        }
    }
}
