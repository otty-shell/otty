//! The Settings popup: Appearance, General and Terminal sections in a wide
//! dialog over every mode, with Reset and Save under them.

use iced::Element;
use iced::widget::{Id, column};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::{BuiltinTheme, ThemeProps};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use crate::event::SettingsIntent;
use crate::model::SettingsViewModel;
use crate::types::LanguageSetting;

/// Widget id of the Shell field.
const SHELL_FIELD: Id = Id::new("settings_shell");
/// Widget id of the Default editor field.
const EDITOR_FIELD: Id = Id::new("settings_editor");

/// Props for the Settings popup.
pub(crate) struct SettingsDialogProps<'a> {
    pub(crate) vm: SettingsViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the Settings popup: the title with the close cross, the
/// Appearance, General and Terminal sections, then Reset and Save. A press on
/// the dimmed area around it closes it like the cross. In a window too small
/// for it, the popup keeps its width and scrolls.
pub(crate) fn view(
    props: SettingsDialogProps<'_>,
) -> Element<'_, SettingsIntent> {
    let palette = props.theme.theme.ui();

    let sections = column![
        appearance_section(&props, palette),
        general_section(&props, palette),
        terminal_section(&props, palette),
    ]
    .spacing(spacing::SPACE_6);
    let (reset, save) = reset_and_save(&props, palette);

    Dialog::new(i18n::t(Key::SettingsTitle), palette)
        .width(size::DIALOG_WIDE)
        .on_close(SettingsIntent::Close, i18n::t(Key::ButtonClose))
        // Owner, 2026-11-09: only Settings closes on a press outside it.
        .on_backdrop_press(SettingsIntent::Close)
        .body(sections)
        .action(reset)
        .action(save)
        .into()
}

/// Reset and Save, which work only while there is something to save.
fn reset_and_save<'a>(
    props: &SettingsDialogProps<'a>,
    palette: &UiPalette,
) -> (Element<'a, SettingsIntent>, Element<'a, SettingsIntent>) {
    let is_enabled = props.vm.is_dirty && !props.vm.is_saving;

    let mut reset =
        Button::new(i18n::t(Key::ButtonReset), palette).kind(ButtonKind::Ghost);
    let mut save = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary);
    if is_enabled {
        reset = reset.on_press(SettingsIntent::Reset);
        save = save.on_press(SettingsIntent::Save);
    }

    (reset.into(), save.into())
}

fn appearance_section<'a>(
    props: &SettingsDialogProps<'a>,
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
    props: &SettingsDialogProps<'a>,
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

/// The Shell and Editor fields and the pane switch. A focused field keeps
/// Esc for itself, so each field closes the popup on Esc too.
fn terminal_section<'a>(
    props: &SettingsDialogProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent> {
    let is_editable = !props.vm.is_saving;

    // The label stays empty: the row around the field already names it.
    let mut shell =
        TextField::new("", props.vm.draft.terminal_shell(), palette)
            .id(SHELL_FIELD)
            .on_escape(SettingsIntent::Close);
    let mut editor =
        TextField::new("", props.vm.draft.terminal_editor(), palette)
            .id(EDITOR_FIELD)
            .on_escape(SettingsIntent::Close);
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
    use iced::keyboard::key::Named;
    use iced::widget::row;
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::AppTheme;
    use otty_ui_kit::size;

    use super::*;
    use crate::types::SettingsData;

    /// The right edge of the popup's content in a window `width` wide that
    /// has room for the whole popup.
    fn content_right(width: f32) -> f32 {
        (width + size::DIALOG_WIDE) / 2.0 - spacing::SPACE_5
    }

    fn props<'a>(
        draft: &'a SettingsData,
        app_theme: &'a AppTheme,
        is_dirty: bool,
        is_saving: bool,
    ) -> SettingsDialogProps<'a> {
        SettingsDialogProps {
            vm: SettingsViewModel {
                draft,
                is_dirty,
                is_saving,
            },
            theme: ThemeProps::new(app_theme),
        }
    }

    /// Draws the popup in a window of the given size.
    fn popup_in<'a>(
        props: SettingsDialogProps<'a>,
        width: f32,
        height: f32,
    ) -> Simulator<'a, SettingsIntent> {
        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            view(props),
        )
    }

    fn click_in_popup(
        draft: &SettingsData,
        is_dirty: bool,
        label: &str,
    ) -> Vec<SettingsIntent> {
        let app_theme = AppTheme::default();
        let mut ui = simulator(view(props(draft, &app_theme, is_dirty, false)));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
    }

    /// Scrolls the popup, under its title, as far as `x` and `y` go.
    fn wheel(ui: &mut Simulator<'_, SettingsIntent>, x: f32, y: f32) {
        let title = ui
            .find(i18n::t(Key::SettingsTitle))
            .expect("the title should exist")
            .visible_bounds()
            .expect("the title should be in view");

        ui.point_at(title.center());
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels { x, y },
            },
        )]);
    }

    /// How many scrollers the popup has.
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

    /// Opens the language list at `x`, level with the middle of its row,
    /// and picks the entry two rows under it. The list shows its text
    /// itself, so the test finds it by place: the control sits at the right
    /// edge of the row. The popup must not be scrolled down.
    fn pick_a_language(ui: &mut Simulator<'_, SettingsIntent>, x: f32) {
        let name = ui.find(i18n::t(Key::FieldLanguage)).expect("row name");
        let note = ui.find(i18n::t(Key::DescLanguage)).expect("row note");
        let (name, note) = (name.bounds(), note.bounds());
        let middle = (name.y + note.y + note.height) / 2.0;

        ui.point_at(iced::Point::new(x, middle));
        let _ = ui.simulate(iced_test::simulator::click());
        ui.point_at(iced::Point::new(x, middle + 2.0 * size::CONTROL_LG));
        let _ = ui.simulate(iced_test::simulator::click());
    }

    #[test]
    fn given_a_small_window_then_the_popup_has_one_scroller() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), 300.0, 300.0);

        assert_eq!(scroller_count(&mut ui), 1);
    }

    #[test]
    fn the_popup_is_titled_settings_and_shows_every_section() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui = simulator(view(props(&draft, &app_theme, false, false)));

        for text in [
            i18n::t(Key::SettingsTitle),
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

    #[test]
    fn given_a_normal_window_then_reset_and_save_end_the_popup_at_its_right() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let width = 1024.0;
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), width, 768.0);

        let save = ui.find("Save").expect("Save should exist").bounds();
        let reset = ui.find("Reset").expect("Reset should exist").bounds();
        let title = ui
            .find(i18n::t(Key::SettingsTitle))
            .expect("the title should exist")
            .bounds();
        let last = ui
            .find(i18n::t(Key::DescEqualizePanes))
            .expect("the last field should exist")
            .bounds();

        assert!(save.y > last.y + last.height, "Save comes after the fields");
        assert_eq!(reset.y, save.y);
        assert!(reset.x + reset.width < save.x, "Reset comes before Save");
        assert!(save.x + save.width <= content_right(width));
        assert!(save.x + save.width > content_right(width) - 20.0);
        assert!(title.y < last.y);
    }

    /// The cross in the title closes the popup.
    #[test]
    fn the_cross_closes_the_popup() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let width = 1024.0;
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), width, 768.0);
        let title = ui
            .find(i18n::t(Key::SettingsTitle))
            .expect("the title should exist")
            .bounds();

        ui.point_at(iced::Point::new(
            content_right(width) - size::CONTROL_MD / 2.0,
            title.center_y(),
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<SettingsIntent> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [SettingsIntent::Close]));
    }

    /// Owner, 2026-11-09: a press on the dimmed area around the popup closes
    /// it like the cross, with any mouse button. Only Settings does this.
    #[test]
    fn a_press_on_the_dimmed_area_closes_the_popup() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();

        for button in [
            iced::mouse::Button::Left,
            iced::mouse::Button::Right,
            iced::mouse::Button::Middle,
        ] {
            let mut ui =
                popup_in(props(&draft, &app_theme, true, false), 1024.0, 768.0);

            ui.point_at(iced::Point::new(20.0, 384.0));
            let _ = ui.simulate([
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button)),
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button)),
            ]);

            let messages: Vec<SettingsIntent> = ui.into_messages().collect();
            assert!(
                matches!(messages.as_slice(), [SettingsIntent::Close]),
                "{button:?}: {messages:?}"
            );
        }
    }

    /// The popup itself is no dimmed area: a press on its text keeps it
    /// open.
    #[test]
    fn a_press_inside_the_popup_does_not_close_it() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), 1024.0, 768.0);

        ui.click(i18n::t(Key::SectionTerminal))
            .expect("the section should show");
        ui.click(i18n::t(Key::DescLanguage))
            .expect("the note should show");

        let messages: Vec<SettingsIntent> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    /// A focused field keeps Esc for itself, so without its own Esc the
    /// field would only lose its focus and the popup would stay.
    #[test]
    fn esc_in_the_shell_or_the_editor_field_closes_the_popup() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();

        for field in [SHELL_FIELD, EDITOR_FIELD] {
            let mut ui =
                simulator(view(props(&draft, &app_theme, false, false)));

            ui.click(field.clone()).expect("the field should show");
            ui.tap_key(Named::Escape);

            let messages: Vec<SettingsIntent> = ui.into_messages().collect();
            assert!(
                matches!(messages.as_slice(), [SettingsIntent::Close]),
                "{field:?}: {messages:?}"
            );
        }
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
                let props = props(&draft, &app_theme, is_dirty, false);
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
                let (reset, save) = reset_and_save(&props, palette);
                let drawn: Element<'_, SettingsIntent> =
                    row![reset, save].spacing(spacing::SPACE_3).into();

                let expected_saved = Simulator::with_size(
                    iced::Settings::default(),
                    window,
                    expected,
                )
                .snapshot(&Theme::Dark)
                .expect("the buttons should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");
                let buttons_match = Simulator::with_size(
                    iced::Settings::default(),
                    window,
                    drawn,
                )
                .snapshot(&Theme::Dark)
                .expect("the buttons should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");

                assert!(expected_saved);
                assert!(buttons_match, "{builtin:?}, dirty {is_dirty}");
            }
        }
    }

    #[test]
    fn given_a_low_window_then_save_comes_after_the_fields_and_is_reached_by_scrolling()
     {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), 1024.0, 200.0);

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
        let mut ui =
            popup_in(props(&draft, &app_theme, false, false), 1024.0, 200.0);

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
        let mut ui =
            popup_in(props(&draft, &app_theme, true, true), 1024.0, 768.0);

        ui.click("Save").expect("Save should be in reach");
        ui.click("Reset").expect("Reset should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_a_normal_window_then_the_language_list_opens_and_picks() {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let width = 1024.0;
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), width, 768.0);

        pick_a_language(&mut ui, content_right(width) - 20.0);
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(
            matches!(messages.as_slice(), [SettingsIntent::LanguageChanged(_)]),
            "{messages:?}"
        );
    }

    /// In a window narrower than the popup, it keeps its width and scrolls
    /// sideways.
    #[test]
    fn given_a_narrow_window_then_the_popup_keeps_its_width_and_save_is_reached_by_scrolling()
     {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), 300.0, 600.0);

        let name = ui
            .find(i18n::t(Key::FieldTheme))
            .expect("row name")
            .bounds();
        let last_choice = ui
            .find("OTTY Light")
            .expect("the theme should show")
            .bounds();
        let hidden = ui.find("Save").expect("Save should exist");
        wheel(&mut ui, -10_000.0, -10_000.0);
        ui.click("Save").expect("Save should be in reach");
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        let width = last_choice.x + last_choice.width - name.x;
        let content = size::DIALOG_WIDE - 2.0 * spacing::SPACE_5;
        assert!(width <= content && width > content - 30.0, "{width}");
        assert_eq!(hidden.visible_bounds(), None);
        assert!(matches!(messages.as_slice(), [SettingsIntent::Save]));
    }

    #[test]
    fn given_a_narrow_window_when_scrolled_then_the_language_list_opens_and_picks()
     {
        let draft = SettingsData::default();
        let app_theme = AppTheme::default();
        let mut ui =
            popup_in(props(&draft, &app_theme, true, false), 300.0, 600.0);

        wheel(&mut ui, -10_000.0, 0.0);
        let last_choice = ui
            .find("OTTY Light")
            .expect("the theme should show")
            .visible_bounds()
            .expect("the theme should be in view");
        pick_a_language(&mut ui, last_choice.x + last_choice.width - 20.0);
        let messages: Vec<SettingsIntent> = ui.into_messages().collect();

        assert!(
            matches!(messages.as_slice(), [SettingsIntent::LanguageChanged(_)]),
            "{messages:?}"
        );
    }

    #[test]
    fn given_dirty_draft_when_save_clicked_then_save_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_popup(&draft, true, "Save");

        assert!(matches!(messages.as_slice(), [SettingsIntent::Save]));
    }

    #[test]
    fn given_clean_draft_when_save_clicked_then_nothing_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_popup(&draft, false, "Save");

        assert!(messages.is_empty());
    }

    #[test]
    fn given_dirty_draft_when_reset_clicked_then_reset_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_popup(&draft, true, "Reset");

        assert!(matches!(messages.as_slice(), [SettingsIntent::Reset]));
    }

    #[test]
    fn given_light_theme_clicked_then_theme_selected_intent_is_sent() {
        let draft = SettingsData::default();

        let messages = click_in_popup(&draft, false, "OTTY Light");

        assert!(matches!(
            messages.as_slice(),
            [SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight)]
        ));
    }
}
