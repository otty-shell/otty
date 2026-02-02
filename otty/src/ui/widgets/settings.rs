use iced::alignment;
use iced::widget::button::Status as ButtonStatus;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Space, button, column, container, mouse_area, row, scrollable, svg,
    text, text_input,
};
use iced::{Background, Color, Element, Length, mouse};
use otty_ui_term::parse_hex_color;

use crate::features::settings::{
    SettingsEvent, SettingsNode, SettingsPreset, SettingsSection,
    SettingsState, is_valid_hex_color, palette_label,
};
use crate::icons;
use crate::theme::{IcedColorPalette, ThemeProps};
use crate::ui::widgets::tree::{TreeNode, flatten_tree};

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) state: &'a SettingsState,
    pub(crate) theme: ThemeProps<'a>,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let header = settings_header(props);
    let content = settings_split_view(props);

    column![header, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_header<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let save_button = action_button(
        "Save",
        props.state.dirty,
        SettingsEvent::Save,
        props.theme,
    );
    let reset_button = action_button(
        "Reset",
        props.state.dirty,
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

fn settings_split_view<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let nav = settings_nav_tree(props);
    let form = settings_form(props);

    row![nav, form]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_nav_tree<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let entries = flatten_tree(&props.state.tree);
    let mut column = Column::new().spacing(0);

    for entry in entries {
        column = column.push(render_nav_row(props, &entry));
    }

    let palette = props.theme.theme.iced_palette().clone();

    let scrollable = scrollable::Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(move |theme, status| {
            let mut style = scrollable::default(theme, status);
            let radius = iced::border::Radius::from(0.0);

            style.vertical_rail.border.radius = radius;
            style.vertical_rail.scroller.border.radius = radius;
            style.horizontal_rail.border.radius = radius;
            style.horizontal_rail.scroller.border.radius = radius;

            let mut scroller_color =
                match style.vertical_rail.scroller.background {
                    Background::Color(color) => color,
                    _ => palette.dim_foreground,
                };
            scroller_color.a = (scroller_color.a * 0.7).min(1.0);
            style.vertical_rail.scroller.background =
                Background::Color(scroller_color);
            style.horizontal_rail.scroller.background =
                Background::Color(scroller_color);

            style
        });

    container(scrollable)
        .width(Length::Fixed(NAV_WIDTH))
        .height(Length::Fill)
        .into()
}

fn render_nav_row<'a>(
    props: Props<'a>,
    entry: &crate::ui::widgets::tree::FlattenedNode<'a, SettingsNode>,
) -> Element<'a, SettingsEvent> {
    let indent = entry.depth as f32 * NAV_INDENT;
    let is_selected = props.state.selected_path == entry.path;

    let icon_view = nav_icon(entry.node)
        .map(|icon| {
            svg_icon(icon, props.theme.theme.iced_palette().dim_foreground)
        })
        .unwrap_or_else(|| {
            container(Space::new())
                .width(Length::Fixed(NAV_ICON_SIZE))
                .height(Length::Fixed(NAV_ICON_SIZE))
                .into()
        });

    let title = container(
        text(entry.node.title())
            .size(NAV_FONT_SIZE)
            .width(Length::Fill)
            .wrapping(Wrapping::None)
            .align_x(alignment::Horizontal::Left),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(alignment::Vertical::Center);

    let leading = row![icon_view]
        .spacing(0)
        .align_y(alignment::Vertical::Center);

    let content =
        row![Space::new().width(Length::Fixed(indent)), leading, title,]
            .spacing(NAV_ROW_SPACING)
            .align_y(alignment::Vertical::Center);

    let palette = props.theme.theme.iced_palette().clone();
    let row = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(NAV_ROW_HEIGHT))
        .padding([0.0, NAV_ROW_PADDING_X])
        .style(move |_| {
            let background = if is_selected {
                let mut color = palette.dim_blue;
                color.a = 0.7;
                Some(color.into())
            } else {
                None
            };
            iced::widget::container::Style {
                background,
                text_color: Some(palette.foreground),
                ..Default::default()
            }
        });

    let path = entry.path.clone();
    mouse_area(row)
        .interaction(mouse::Interaction::Pointer)
        .on_press(SettingsEvent::NodePressed { path })
        .into()
}

fn settings_form<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let content = match props.state.selected_section {
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
        .style(move |theme, status| {
            let mut style = scrollable::default(theme, status);
            let radius = iced::border::Radius::from(0.0);

            style.vertical_rail.border.radius = radius;
            style.vertical_rail.scroller.border.radius = radius;
            style.horizontal_rail.border.radius = radius;
            style.horizontal_rail.scroller.border.radius = radius;

            let mut scroller_color =
                match style.vertical_rail.scroller.background {
                    Background::Color(color) => color,
                    _ => palette.dim_foreground,
                };
            scroller_color.a = (scroller_color.a * 0.7).min(1.0);
            style.vertical_rail.scroller.background =
                Background::Color(scroller_color);
            style.horizontal_rail.scroller.background =
                Background::Color(scroller_color);

            style
        });

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn terminal_form<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let shell_input = text_input("", &props.state.draft.terminal.shell)
        .on_input(SettingsEvent::ShellChanged)
        .padding([FORM_INPUT_PADDING_Y, FORM_INPUT_PADDING_X])
        .size(FORM_INPUT_FONT_SIZE)
        .width(Length::Fill)
        .style(text_input_style(props.theme));

    let editor_input = text_input("", &props.state.draft.terminal.editor)
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

fn theme_form<'a>(props: Props<'a>) -> Element<'a, SettingsEvent> {
    let preset_button = action_button(
        "OTTY Dark",
        true,
        SettingsEvent::ApplyPreset(SettingsPreset::OttyDark),
        props.theme,
    );
    let presets = row![preset_button].spacing(HEADER_BUTTON_SPACING);

    let mut palette_column = Column::new().spacing(PALETTE_ROW_SPACING);
    for (index, value) in props.state.palette_inputs.iter().enumerate() {
        let label = palette_label(index).map_or_else(
            || {
                let index_display = index + 1;
                format!("Color {index_display}")
            },
            |label| label.to_string(),
        );
        let label = text(label)
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
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2),
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
        column![section_title("Theme", props.theme), presets, palette_column,]
            .spacing(FORM_SECTION_SPACING)
            .padding(FORM_PADDING);

    content.into()
}

fn section_title<'a>(
    title: &'a str,
    theme: ThemeProps<'a>,
) -> Element<'a, SettingsEvent> {
    let palette = theme.theme.iced_palette();
    text(title)
        .size(HEADER_FONT_SIZE)
        .style(move |_| iced::widget::text::Style {
            color: Some(palette.dim_foreground),
        })
        .into()
}

fn form_row<'a>(
    label: &'a str,
    input: iced::widget::TextInput<'a, SettingsEvent>,
) -> Element<'a, SettingsEvent> {
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
) -> Element<'a, SettingsEvent> {
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
) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |base: &iced::Theme, status| {
        let mut style = iced::widget::text_input::default(base, status);
        style.selection = palette.blue;
        style
    }
}

fn nav_icon(node: &SettingsNode) -> Option<&'static [u8]> {
    if node.is_folder() {
        Some(if node.expanded {
            icons::FOLDER_OPENED
        } else {
            icons::FOLDER
        })
    } else {
        None
    }
}

fn svg_icon<'a>(
    icon: &'static [u8],
    color: Color,
) -> Element<'a, SettingsEvent> {
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
        &self.title
    }

    fn children(&self) -> Option<&[Self]> {
        if self.is_folder() {
            Some(&self.children)
        } else {
            None
        }
    }

    fn expanded(&self) -> bool {
        self.expanded
    }

    fn is_folder(&self) -> bool {
        self.is_folder()
    }
}
