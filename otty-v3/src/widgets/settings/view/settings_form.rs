use iced::widget::button::Status as ButtonStatus;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Space, button, column, container, row, scrollable, svg, text,
    text_input,
};
use iced::{Color, Element, Length, Theme, alignment};
use otty_ui_term::parse_hex_color;
use otty_ui_tree::{TreeNode, TreeRowContext, TreeView};

use crate::shared::ui::theme::{IcedColorPalette, ThemeProps};
use crate::shared::ui::tree_style;
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::settings::model::{
    SettingsNode, SettingsPreset, SettingsSection, SettingsViewModel,
    is_valid_hex_color, palette_label,
};

const HEADER_HEIGHT: f32 = 32.0;
const HEADER_PADDING_X: f32 = 12.0;
const HEADER_FONT_SIZE: f32 = 12.0;
const HEADER_BUTTON_HEIGHT: f32 = 22.0;
const HEADER_BUTTON_PADDING_X: f32 = 10.0;
const HEADER_BUTTON_SPACING: f32 = 8.0;

const NAV_WIDTH: f32 = 220.0;
const NAV_ROW_HEIGHT: f32 = 24.0;
const NAV_FONT_SIZE: f32 = 12.0;
const NAV_INDENT: f32 = 14.0;
const NAV_ICON_SIZE: f32 = 14.0;
const NAV_ROW_PADDING_X: f32 = 8.0;
const NAV_ROW_SPACING: f32 = 6.0;

const FORM_PADDING: f32 = 16.0;
const FORM_SECTION_SPACING: f32 = 16.0;
const FORM_ROW_SPACING: f32 = 10.0;
const FORM_LABEL_WIDTH: f32 = 160.0;
const FORM_INPUT_HEIGHT: f32 = 28.0;
const FORM_INPUT_PADDING_X: f32 = 8.0;
const FORM_INPUT_PADDING_Y: f32 = 4.0;
const FORM_INPUT_FONT_SIZE: f32 = 12.0;

const PALETTE_ROW_SPACING: f32 = 8.0;
const PALETTE_SWATCH_SIZE: f32 = 16.0;
const PALETTE_SWATCH_BORDER: f32 = 1.0;

/// Props for the settings form view.
pub(crate) struct SettingsFormProps<'a> {
    pub(crate) vm: SettingsViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the full settings view (header + nav + form).
pub(crate) fn view(
    props: SettingsFormProps<'_>,
) -> Element<'_, SettingsEvent, Theme, iced::Renderer> {
    let header = settings_header(&props);
    let content = settings_split_view(&props);

    column![header, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_header<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let save_button = action_button(
        "Save",
        props.vm.is_dirty,
        SettingsEvent::Save,
        props.theme,
    );
    let reset_button = action_button(
        "Reset",
        props.vm.is_dirty,
        SettingsEvent::Reset,
        props.theme,
    );

    let actions =
        row![save_button, reset_button].spacing(HEADER_BUTTON_SPACING);

    let palette = props.theme.theme.iced_palette().clone();

    container(actions)
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT))
        .padding([0.0, HEADER_PADDING_X])
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.overlay.into()),
            text_color: Some(palette.foreground),
            ..Default::default()
        })
        .into()
}

fn settings_split_view<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let nav = settings_nav_tree(props);
    let form = settings_form(props);

    row![nav, form]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_nav_tree<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette().clone();
    let row_palette = palette.clone();
    let icon_color = palette.dim_foreground;

    let tree_view =
        TreeView::new(props.vm.tree, move |context| render_nav_row(context))
            .selected_row(Some(props.vm.selected_path))
            .hovered_row(props.vm.hovered_path)
            .on_press(|path| SettingsEvent::NodePressed { path })
            .on_hover(|path| SettingsEvent::NodeHovered { path })
            .row_style(move |context| nav_row_style(&row_palette, context))
            .row_leading_content(move |context| {
                nav_toggle_icon(context, icon_color)
            })
            .indent_size(NAV_INDENT)
            .spacing(0.0);

    let scroll_palette = palette.clone();
    let scrollable = scrollable::Scrollable::new(tree_view.view())
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(tree_style::thin_scroll_style(scroll_palette));

    container(scrollable)
        .width(Length::Fixed(NAV_WIDTH))
        .height(Length::Fill)
        .into()
}

fn render_nav_row<'a>(
    context: &TreeRowContext<'a, SettingsNode>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let title = text(context.entry.node.title())
        .size(NAV_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    container(title)
        .width(Length::Fill)
        .height(Length::Fixed(NAV_ROW_HEIGHT))
        .padding([0.0, NAV_ROW_PADDING_X])
        .align_y(alignment::Vertical::Center)
        .into()
}

fn settings_form<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let content: Element<'a, SettingsEvent, Theme, iced::Renderer> =
        match props.vm.selected_section {
            SettingsSection::Terminal => terminal_form(props),
            SettingsSection::Theme => theme_form(props),
        };

    let palette = props.theme.theme.iced_palette().clone();

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(tree_style::thin_scroll_style(palette));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn terminal_form<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let shell_input = text_input("", props.vm.draft.terminal_shell())
        .on_input(SettingsEvent::ShellChanged)
        .padding([FORM_INPUT_PADDING_Y, FORM_INPUT_PADDING_X])
        .size(FORM_INPUT_FONT_SIZE)
        .width(Length::Fill)
        .style(text_input_style(props.theme));

    let editor_input = text_input("", props.vm.draft.terminal_editor())
        .on_input(SettingsEvent::EditorChanged)
        .padding([FORM_INPUT_PADDING_Y, FORM_INPUT_PADDING_X])
        .size(FORM_INPUT_FONT_SIZE)
        .width(Length::Fill)
        .style(text_input_style(props.theme));

    let content = column![
        section_title("Terminal", props.theme),
        form_row("Shell", shell_input),
        form_row("Default editor", editor_input),
    ]
    .spacing(FORM_SECTION_SPACING)
    .padding(FORM_PADDING);

    content.into()
}

fn theme_form<'a>(
    props: &SettingsFormProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let preset_button = action_button(
        "OTTY Dark",
        true,
        SettingsEvent::ApplyPreset(SettingsPreset::OttyDark),
        props.theme,
    );
    let presets = row![preset_button].spacing(HEADER_BUTTON_SPACING);

    let mut palette_column = Column::new().spacing(PALETTE_ROW_SPACING);
    for (index, value) in props.vm.palette_inputs.iter().enumerate() {
        let label_text = palette_label(index).map_or_else(
            || {
                let index_display = index + 1;
                format!("Color {index_display}")
            },
            |label| label.to_string(),
        );
        let label = text(label_text)
            .size(FORM_INPUT_FONT_SIZE)
            .width(Length::Fixed(FORM_LABEL_WIDTH))
            .align_x(alignment::Horizontal::Left)
            .wrapping(Wrapping::None);

        let input = text_input("", value)
            .on_input(move |value| SettingsEvent::PaletteChanged {
                index,
                value,
            })
            .padding([FORM_INPUT_PADDING_Y, FORM_INPUT_PADDING_X])
            .size(FORM_INPUT_FONT_SIZE)
            .width(Length::Fill)
            .style(text_input_style(props.theme));

        let swatch_color = if is_valid_hex_color(value) {
            parse_hex_color(value)
        } else {
            props.theme.theme.iced_palette().dim_black
        };

        let swatch = container(Space::new())
            .width(Length::Fixed(PALETTE_SWATCH_SIZE))
            .height(Length::Fixed(PALETTE_SWATCH_SIZE))
            .style(move |_| iced::widget::container::Style {
                background: Some(swatch_color.into()),
                border: iced::Border {
                    width: PALETTE_SWATCH_BORDER,
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                    radius: iced::border::Radius::from(2.0),
                },
                ..Default::default()
            });

        let row = row![label, input, swatch]
            .spacing(NAV_ROW_SPACING)
            .align_y(alignment::Vertical::Center);

        palette_column = palette_column.push(row);
    }

    let content =
        column![section_title("Theme", props.theme), presets, palette_column]
            .spacing(FORM_SECTION_SPACING)
            .padding(FORM_PADDING);

    content.into()
}

fn section_title<'a>(
    title: &'a str,
    theme: ThemeProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette();
    let color = palette.dim_foreground;
    text(title)
        .size(HEADER_FONT_SIZE)
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into()
}

fn form_row<'a>(
    label: &'a str,
    input: iced::widget::TextInput<'a, SettingsEvent, Theme, iced::Renderer>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let label = text(label)
        .size(FORM_INPUT_FONT_SIZE)
        .width(Length::Fixed(FORM_LABEL_WIDTH))
        .align_x(alignment::Horizontal::Left)
        .wrapping(Wrapping::None);

    row![label, input]
        .spacing(FORM_ROW_SPACING)
        .align_y(alignment::Vertical::Center)
        .height(Length::Fixed(FORM_INPUT_HEIGHT))
        .into()
}

fn action_button<'a>(
    label: &'a str,
    enabled: bool,
    event: SettingsEvent,
    theme: ThemeProps<'a>,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette().clone();
    let content = container(
        text(label)
            .size(HEADER_FONT_SIZE)
            .align_x(alignment::Horizontal::Center),
    )
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center);

    let mut button = button(content)
        .padding([0.0, HEADER_BUTTON_PADDING_X])
        .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
        .style(move |_, status| button_style(&palette, status, enabled));

    if enabled {
        button = button.on_press(event);
    }

    button.into()
}

fn button_style(
    palette: &IcedColorPalette,
    status: ButtonStatus,
    enabled: bool,
) -> iced::widget::button::Style {
    let base_color = if enabled {
        match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => palette.dim_blue,
            _ => palette.overlay,
        }
    } else {
        let mut color = palette.overlay;
        color.a = 0.4;
        color
    };

    let text_color = if enabled {
        match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => palette.dim_black,
            _ => palette.foreground,
        }
    } else {
        palette.dim_foreground
    };

    iced::widget::button::Style {
        background: Some(base_color.into()),
        text_color,
        border: iced::Border {
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn text_input_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |base: &Theme, status| {
        let mut style = iced::widget::text_input::default(base, status);
        style.selection = palette.blue;
        style
    }
}

fn nav_row_style(
    palette: &IcedColorPalette,
    context: &TreeRowContext<'_, SettingsNode>,
) -> container::Style {
    tree_style::tree_row_style(palette, context.is_selected, context.is_hovered)
}

fn nav_toggle_icon<'a>(
    context: &TreeRowContext<'a, SettingsNode>,
    color: Color,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    nav_icon(context.entry.node)
        .map(|icon| svg_icon(icon, color))
        .unwrap_or_else(|| Space::new().into())
}

fn nav_icon(node: &SettingsNode) -> Option<&'static [u8]> {
    if node.is_folder() {
        Some(if node.is_expanded() {
            crate::shared::ui::icons::FOLDER_OPENED
        } else {
            crate::shared::ui::icons::FOLDER
        })
    } else {
        None
    }
}

fn svg_icon<'a>(
    icon: &'static [u8],
    color: Color,
) -> Element<'a, SettingsEvent, Theme, iced::Renderer> {
    let handle = svg::Handle::from_memory(icon);
    let svg_icon = svg::Svg::new(handle)
        .width(Length::Fixed(NAV_ICON_SIZE))
        .height(Length::Fixed(NAV_ICON_SIZE))
        .style(move |_, _| svg::Style { color: Some(color) });

    container(svg_icon)
        .width(Length::Fixed(NAV_ICON_SIZE))
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

impl TreeNode for SettingsNode {
    fn title(&self) -> &str {
        SettingsNode::title(self)
    }

    fn children(&self) -> Option<&[Self]> {
        if SettingsNode::is_folder(self) {
            Some(SettingsNode::children(self))
        } else {
            None
        }
    }

    fn expanded(&self) -> bool {
        self.is_expanded()
    }

    fn is_folder(&self) -> bool {
        SettingsNode::is_folder(self)
    }
}
