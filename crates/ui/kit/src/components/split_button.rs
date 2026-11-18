//! `SplitButton`: primary icon button with a narrow chevron for a menu.

use iced::widget::{button, container, row, svg};
use iced::{Alignment, Border, Element, Length};

use super::icon_button::{IconButton, IconButtonSize, IconTone};
use super::icon_hover::IconHover;
use super::tooltip::tooltip;
use super::{glyph, icon_opacity};
use crate::palette::UiPalette;
use crate::{radius, size};

/// Edge of the chevron (`width: 11px; height: 11px`).
const CHEVRON_SIZE: f32 = 11.0;

/// Primary icon button and a chevron button side by side in one
/// `radius-sm` frame. The frame shows `surface-raised` while the menu opened
/// by the chevron is open. A part without a message is disabled: its icon
/// is drawn faded and does not light up under the pointer; its tooltip
/// still shows.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::split_button::SplitButton;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     NewSession,
///     ShowMenu,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let plus = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let split: Element<'_, Message> = SplitButton::new(
///     plus,
///     "New terminal",
///     "More ways to start a session",
///     &palette,
/// )
/// .on_press(Message::NewSession)
/// .on_menu(Message::ShowMenu)
/// .into();
/// ```
pub struct SplitButton<Message> {
    icon: svg::Handle,
    label: String,
    menu_label: String,
    palette: UiPalette,
    is_menu_open: bool,
    on_press: Option<Message>,
    on_menu: Option<Message>,
}

impl<Message> SplitButton<Message> {
    /// Creates a split button; `label` and `menu_label` are the tooltips of
    /// the primary and the chevron part.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        menu_label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            menu_label: menu_label.into(),
            palette: palette.clone(),
            is_menu_open: false,
            on_press: None,
            on_menu: None,
        }
    }

    /// Shows the frame background while the menu is open.
    pub fn menu_open(mut self, is_menu_open: bool) -> Self {
        self.is_menu_open = is_menu_open;
        self
    }

    /// Sets the message sent when the primary part is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent when the chevron is pressed.
    pub fn on_menu(mut self, message: Message) -> Self {
        self.on_menu = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SplitButton<Message>>
    for Element<'a, Message>
{
    fn from(value: SplitButton<Message>) -> Self {
        let palette = value.palette;
        let frame = palette.surface_raised();
        let is_menu_open = value.is_menu_open;

        let mut primary = IconButton::new(value.icon, value.label, &palette)
            .size(IconButtonSize::Small)
            .tone(IconTone::Accent);
        if let Some(message) = value.on_press {
            primary = primary.on_press(message);
        }

        // iced drops the alpha of an svg's style color, so a disabled
        // chevron keeps its idle color and fades through the svg's opacity.
        let is_menu_enabled = value.on_menu.is_some();
        let muted = palette.text_muted();
        let lit = if is_menu_enabled {
            palette.text_strong()
        } else {
            muted
        };
        let raised = palette.surface_raised();
        let pointer = IconHover::new();
        let chevron = button(
            container(
                svg(glyph::caret(true))
                    .width(CHEVRON_SIZE)
                    .height(CHEVRON_SIZE)
                    .opacity(icon_opacity(is_menu_enabled))
                    .style(pointer.icon_style(muted, lit)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
        )
        .width(size::SPLIT_CHEVRON)
        .height(size::CONTROL_SM)
        .padding(0)
        .on_press_maybe(value.on_menu)
        .style(move |_, status| button::Style {
            background: matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            )
            .then(|| raised.into()),
            border: Border {
                radius: radius::RADIUS_SM.into(),
                ..Border::default()
            },
            ..button::Style::default()
        });
        let chevron =
            tooltip(pointer.watch(chevron), value.menu_label, &palette);

        container(row![primary, chevron].align_y(Alignment::Center))
            .style(move |_| container::Style {
                background: is_menu_open.then(|| frame.into()),
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;
    use crate::testing::pictures::{
        SMALL_WINDOW, assert_icon_lights_up, draws,
    };
    use crate::testing::temp_dir::TempDir;

    #[test]
    fn given_pointer_on_chevron_corner_then_chevron_is_lit_like_over_the_icon()
    {
        let view = || {
            SplitButton::new(
                glyph::plus(),
                "New",
                "More",
                &UiPalette::otty_dark(),
            )
            .on_press(())
            .on_menu(())
            .into()
        };

        assert_icon_lights_up(
            "split-chevron",
            view,
            iced::Rectangle::new(
                iced::Point::new(size::CONTROL_SM, 0.0),
                iced::Size::new(size::SPLIT_CHEVRON, size::CONTROL_SM),
            ),
        );
    }

    /// Design system: with no messages the arrow is drawn at 45 % of its
    /// idle color like the plus, does not light up under the pointer and
    /// gets no hover background, in both themes; its tooltip still shows.
    /// The pictures are compared with a disabled plus and a bare arrow
    /// drawn that way, because iced paints an svg with the red, green and
    /// blue of its style color only and drops the alpha.
    #[test]
    fn given_no_messages_then_the_arrow_is_drawn_at_45_percent_and_not_lit() {
        let folder = TempDir::new("split-button-disabled");
        let arrow = iced::Point::new(
            size::CONTROL_SM + size::SPLIT_CHEVRON / 2.0,
            size::CONTROL_SM / 2.0,
        );

        for (name, palette) in [
            ("dark", UiPalette::otty_dark()),
            ("light", UiPalette::otty_light()),
        ] {
            let idle = folder.path().join(format!("{name}-idle"));
            let hovered = folder.path().join(format!("{name}-hovered"));
            let faded = {
                let palette = palette.clone();

                move || faded_split(&palette)
            };
            let split = |is_enabled: bool| {
                let palette = palette.clone();

                move || {
                    let split = SplitButton::new(
                        glyph::plus(),
                        "New",
                        "More",
                        &palette,
                    );
                    if !is_enabled {
                        return split.into();
                    }

                    split.on_press(()).on_menu(()).into()
                }
            };

            assert!(draws(&faded, SMALL_WINDOW, None, &idle), "{name}: save");
            assert!(
                draws(&faded, SMALL_WINDOW, Some(arrow), &hovered),
                "{name}"
            );
            assert!(
                draws(&split(false), SMALL_WINDOW, None, &idle),
                "{name}: idle"
            );
            assert!(
                draws(&split(false), SMALL_WINDOW, Some(arrow), &hovered),
                "{name}: under the pointer"
            );
            assert!(
                !draws(&split(true), SMALL_WINDOW, None, &idle),
                "{name}: an enabled split button must look different"
            );
        }
    }

    /// A disabled small accent plus, then the arrow in `text-muted` at
    /// 45 % in the middle of its part, with the tooltip "More".
    fn faded_split(palette: &UiPalette) -> Element<'static, ()> {
        let plus = IconButton::new(glyph::plus(), "New", palette)
            .size(IconButtonSize::Small)
            .tone(IconTone::Accent);
        let muted = palette.text_muted();
        let arrow = svg(glyph::caret(true))
            .width(CHEVRON_SIZE)
            .height(CHEVRON_SIZE)
            .opacity(0.45)
            .style(move |_, _| svg::Style { color: Some(muted) });
        let part = container(arrow)
            .center_x(size::SPLIT_CHEVRON)
            .center_y(size::CONTROL_SM);
        let arrow = tooltip(part, "More".to_owned(), palette);

        row![plus, arrow].align_y(Alignment::Center).into()
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        New,
        Menu,
    }

    fn click_at(x: f32) -> Vec<Message> {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            SplitButton::new(glyph::plus(), "New", "More", &palette)
                .on_press(Message::New)
                .on_menu(Message::Menu),
        );

        ui.point_at(Point::new(x, size::CONTROL_SM / 2.0));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn given_primary_part_when_pressed_then_only_new_is_sent() {
        assert_eq!(click_at(size::CONTROL_SM / 2.0), vec![Message::New]);
    }

    #[test]
    fn given_chevron_part_when_pressed_then_only_menu_is_sent() {
        let x = size::CONTROL_SM + size::SPLIT_CHEVRON / 2.0;

        assert_eq!(click_at(x), vec![Message::Menu]);
    }
}
