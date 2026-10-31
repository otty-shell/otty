use iced::widget::{Space, row, text};
use iced::{Element, Length, Theme, alignment};
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
/// one page, with Reset and Save beside the title.
pub(crate) fn view(
    props: SettingsFormProps<'_>,
) -> Element<'_, SettingsIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.ui();

    FormPage::new(palette)
        .section(page_title(&props, palette))
        .section(appearance_section(&props, palette))
        .section(general_section(&props, palette))
        .section(terminal_section(&props, palette))
        .into()
}

/// Page title with the Reset and Save buttons on its right.
fn page_title<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
    let style = typography::TITLE;
    let is_enabled = props.vm.is_dirty && !props.vm.is_saving;

    let mut save = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary);
    let mut reset = Button::new(i18n::t(Key::ButtonReset), palette);
    if is_enabled {
        save = save.on_press(SettingsIntent::Save);
        reset = reset.on_press(SettingsIntent::Reset);
    }

    row![
        text(i18n::t(Key::SettingsTabTitle))
            .size(style.size())
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        Space::new().width(Length::Fill),
        reset,
        save,
    ]
    .spacing(spacing::SPACE_3)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn appearance_section<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
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
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
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
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
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
    use iced_test::simulator;

    use super::*;
    use crate::theme::{AppTheme, BuiltinTheme};
    use crate::widgets::settings::types::SettingsData;

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
        let mut ui = simulator(view(props));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
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
        let mut ui = simulator(view(props));

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
