use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Element, Length, Padding, Theme, alignment};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::components::{divider, scrollbar};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use super::super::event::SettingsIntent;
use super::super::model::SettingsViewModel;
use super::super::types::{LanguageSetting, SettingsNode, SettingsSection};
use crate::i18n::{self, Key};
use crate::theme::{BuiltinTheme, ThemeProps};

/// Width of the settings navigation tree. No size token: the design system
/// panel (`panel`, 260px) is wider than this short list needs.
const NAV_WIDTH: f32 = 220.0;
/// Widest the form grows on large windows, so rows stay readable. No size
/// token.
const FORM_MAX_WIDTH: f32 = 720.0;

/// Props for the settings form view.
pub(crate) struct SettingsFormProps<'a> {
    pub(crate) vm: SettingsViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the full settings view (navigation + form).
pub(crate) fn view(
    props: SettingsFormProps<'_>,
) -> Element<'_, SettingsIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.ui();
    let nav = settings_nav_tree(&props, palette);
    let separator = divider::vertical(palette);
    let form = settings_form(&props, palette);

    row![nav, separator, form]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_nav_tree<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
    let row_palette = palette.clone();

    let tree_view = TreeView::new(props.vm.tree, move |context| {
        render_nav_row(context, &row_palette)
    })
    .selected_row(Some(props.vm.selected_path))
    .hovered_row(props.vm.hovered_path)
    .on_press(|path| SettingsIntent::NodePressed { path })
    .on_hover(|path| SettingsIntent::NodeHovered { path })
    .spacing(0.0)
    .keep_source_order();

    let scrollable = scrollable::Scrollable::new(tree_view.view())
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    container(scrollable)
        .width(Length::Fixed(NAV_WIDTH))
        .height(Length::Fill)
        .into()
}

fn render_nav_row<'a>(
    context: &TreeRowContext<'a, SettingsNode>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
    let mut row = TreeRow::new(context.entry.node.title(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered);

    if context.entry.node.is_folder() {
        row = row.caret(context.entry.node.is_expanded());
    }

    row.into()
}

fn settings_form<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
    let section: Element<'a, SettingsIntent, Theme, iced::Renderer> =
        match props.vm.selected_section {
            SettingsSection::General => general_section(props, palette),
            SettingsSection::Appearance => appearance_section(props, palette),
            _ => terminal_section(props, palette),
        };

    let content = column![page_title(props, palette), section]
        .spacing(spacing::SPACE_6)
        .max_width(FORM_MAX_WIDTH)
        .padding(Padding {
            top: spacing::SPACE_7,
            right: spacing::SPACE_7,
            bottom: spacing::SPACE_7,
            left: spacing::SPACE_7,
        });

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
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
                .control(language_selector),
        )
        .into()
}

fn terminal_section<'a>(
    props: &SettingsFormProps<'a>,
    palette: &UiPalette,
) -> Element<'a, SettingsIntent, Theme, iced::Renderer> {
    let is_editable = !props.vm.is_saving;

    let mut shell = TextField::new(
        i18n::t(Key::FieldShell),
        props.vm.draft.terminal_shell(),
        palette,
    );
    let mut editor = TextField::new(
        i18n::t(Key::FieldDefaultEditor),
        props.vm.draft.terminal_editor(),
        palette,
    );
    let mut equalize_panes =
        Switch::new(props.vm.draft.equalize_panes(), palette);
    if is_editable {
        shell = shell.on_input(SettingsIntent::ShellChanged);
        editor = editor.on_input(SettingsIntent::EditorChanged);
        equalize_panes =
            equalize_panes.on_toggle(SettingsIntent::EqualizePanesToggled);
    }

    FormSection::new(i18n::t(Key::SectionTerminal), palette)
        .field(shell)
        .field(editor)
        .field(
            SettingRow::new(i18n::t(Key::FieldPaneLayout), palette)
                .description(i18n::t(Key::OptionEqualizePanes))
                .control(equalize_panes),
        )
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
                .control(theme_selector),
        )
        .into()
}
