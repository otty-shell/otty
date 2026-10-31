//! `TreeRow`: one row of a folder or session tree.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::glyph;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Caret column width (`.ot-tree-caret { width: 12px }`).
const CARET_WIDTH: f32 = 12.0;
/// Gap between caret, label and count (`.ot-tree { gap: 7px }`).
const GAP: f32 = 7.0;
/// Edge of the optional leading icon.
const ICON_SIZE: f32 = 14.0;
/// Horizontal padding of the row (`padding: 0 8px`).
const PADDING_X: f32 = spacing::SPACE_3;

/// Row of a tree. A row without a message is shown static.
///
/// The label is cut at the row edge: `iced` has no text ellipsis.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tree_row::TreeRow;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggle,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let folder: Element<'_, Message> = TreeRow::new("Servers", &palette)
///     .depth(1)
///     .caret(true)
///     .count(4)
///     .on_press(Message::Toggle)
///     .into();
/// ```
pub struct TreeRow<Message> {
    label: String,
    palette: UiPalette,
    depth: u16,
    caret: Option<bool>,
    count: Option<usize>,
    is_selected: bool,
    is_hovered: bool,
    is_drop_target: bool,
    icon: Option<(svg::Handle, Option<Color>)>,
    on_press: Option<Message>,
}

impl<Message> TreeRow<Message> {
    /// Creates a root-level row without caret and count.
    pub fn new(label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            label: label.into(),
            palette: palette.clone(),
            depth: 0,
            caret: None,
            count: None,
            is_selected: false,
            is_hovered: false,
            is_drop_target: false,
            icon: None,
            on_press: None,
        }
    }

    /// Sets the nesting level; each level indents the row by 8px.
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Shows a caret for a row that can open; `is_open` picks its direction.
    /// Rows without a caret keep the caret column empty so labels line up.
    pub fn caret(mut self, is_open: bool) -> Self {
        self.caret = Some(is_open);
        self
    }

    /// Shows the number of children on the right.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Marks the row as the selected one.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Shows the hover background. Use it for a row without a message whose
    /// hover is tracked by the caller.
    pub fn hovered(mut self, is_hovered: bool) -> Self {
        self.is_hovered = is_hovered;
        self
    }

    /// Marks the row as the target of a drag and drop (`accent` background).
    pub fn drop_target(mut self, is_drop_target: bool) -> Self {
        self.is_drop_target = is_drop_target;
        self
    }

    /// Shows an icon before the label; `color` defaults to `text-muted`.
    pub fn icon(mut self, icon: svg::Handle, color: Option<Color>) -> Self {
        self.icon = Some((icon, color));
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<TreeRow<Message>> for Element<'a, Message> {
    fn from(value: TreeRow<Message>) -> Self {
        let palette = value.palette;
        let is_selected = value.is_selected;
        let is_hovered = value.is_hovered;
        let is_drop_target = value.is_drop_target;
        let body = typography::BODY;

        let muted = palette.text_muted();
        let caret: Element<'a, Message> = match value.caret {
            Some(is_open) => svg(glyph::caret(is_open))
                .width(CARET_WIDTH)
                .height(CARET_WIDTH)
                .style(move |_, _| svg::Style { color: Some(muted) })
                .into(),
            None => Space::new().width(CARET_WIDTH).into(),
        };

        let mut content = row![
            Space::new().width(f32::from(value.depth) * size::TREE_INDENT),
            caret,
        ]
        .spacing(GAP)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some((handle, color)) = value.icon {
            let color = color.unwrap_or(muted);
            content = content.push(
                svg(handle)
                    .width(ICON_SIZE)
                    .height(ICON_SIZE)
                    .style(move |_, _| svg::Style { color: Some(color) }),
            );
        }
        content = content.push(
            container(
                text(value.label)
                    .size(body.size())
                    .font(fonts::UI)
                    .wrapping(Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        );
        if let Some(count) = value.count {
            let caption = typography::CAPTION;
            content = content.push(
                text(count.to_string())
                    .size(caption.size())
                    .font(fonts::UI)
                    .color(muted),
            );
        }

        button(content)
            .width(Length::Fill)
            .height(size::ROW)
            .padding(Padding::from([0.0, PADDING_X]))
            .on_press_maybe(value.on_press)
            .style(move |_, status| {
                let hovered = is_hovered || status == button::Status::Hovered;

                button::Style {
                    background: if is_drop_target {
                        Some(palette.accent().into())
                    } else if is_selected {
                        Some(palette.surface_selected().into())
                    } else {
                        hovered.then(|| palette.surface_raised().into())
                    },
                    text_color: if is_drop_target {
                        palette.on_accent()
                    } else if is_selected {
                        palette.text_strong()
                    } else {
                        palette.text_body()
                    },
                    border: Border {
                        radius: radius::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                }
            })
            .into()
    }
}
