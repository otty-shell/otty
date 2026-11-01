//! `ModeSwitcher`: row of icon tabs that picks the active mode.

use iced::widget::{button, container, row, svg, tooltip};
use iced::{Border, Color, Element, Length};

use super::{glyph, icon_button};
use crate::palette::UiPalette;
use crate::{radius, size, spacing};

/// Padding of the group (`padding: 3px`).
const GROUP_PADDING: f32 = 3.0;
/// Gap between tabs (`gap: 2px`).
const TAB_GAP: f32 = 2.0;
/// Icon edge length inside a tab. 16 keeps the icon on whole pixels at
/// common display scales (see `IconButton`).
const ICON_SIZE: f32 = 16.0;
/// Width of the outline of a temporary tab.
const OUTLINE_WIDTH: f32 = 1.0;

/// One tab of a [`ModeSwitcher`].
pub struct ModeTab<Message> {
    icon: svg::Handle,
    label: String,
    shortcut: Option<String>,
    is_active: bool,
    is_temporary: bool,
    on_press: Option<Message>,
}

impl<Message> ModeTab<Message> {
    /// Creates a tab with an `icon`; `label` is the mode name shown in the
    /// tooltip.
    pub fn new(icon: svg::Handle, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            shortcut: None,
            is_active: false,
            is_temporary: false,
            on_press: None,
        }
    }

    /// Shows the shortcut after the name in the tooltip.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Marks the tab as the active mode: `surface-hover` and an accent icon.
    pub fn active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Marks the tab as temporary (the active mode is not pinned): draws a
    /// 1px `line` outline.
    pub fn temporary(mut self, is_temporary: bool) -> Self {
        self.is_temporary = is_temporary;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

/// The trailing "⋯" button that opens the All modes menu.
struct MoreButton<Message> {
    label: String,
    is_open: bool,
    on_press: Message,
}

/// Raised group of icon tabs with a trailing "⋯" button.
///
/// The caller passes the tabs in display order: Terminal, the pinned modes,
/// then the temporary tab of the active mode if it is not pinned.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::mode_switcher::{ModeSwitcher, ModeTab};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Pick(&'static str),
///     ToggleMenu,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let switcher: Element<'_, Message> = ModeSwitcher::new(&palette)
///     .tab(
///         ModeTab::new(icon, "Terminal")
///             .shortcut("Ctrl Shift 1")
///             .active(true)
///             .on_press(Message::Pick("terminal")),
///     )
///     .more("All modes", false, Message::ToggleMenu)
///     .into();
/// ```
pub struct ModeSwitcher<Message> {
    palette: UiPalette,
    tabs: Vec<ModeTab<Message>>,
    more: Option<MoreButton<Message>>,
}

impl<Message> ModeSwitcher<Message> {
    /// Creates an empty switcher.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            tabs: Vec::new(),
            more: None,
        }
    }

    /// Appends a tab.
    pub fn tab(mut self, tab: ModeTab<Message>) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Adds the trailing "⋯" button. `label` is its tooltip; `is_open` turns
    /// it `surface-hover` while the All modes menu is shown.
    pub fn more(
        mut self,
        label: impl Into<String>,
        is_open: bool,
        on_press: Message,
    ) -> Self {
        self.more = Some(MoreButton {
            label: label.into(),
            is_open,
            on_press,
        });
        self
    }
}

impl<'a, Message: Clone + 'a> From<ModeSwitcher<Message>>
    for Element<'a, Message>
{
    fn from(value: ModeSwitcher<Message>) -> Self {
        let palette = value.palette;

        let mut tabs = row![].spacing(TAB_GAP);
        for tab in value.tabs {
            let hint = match &tab.shortcut {
                Some(shortcut) => format!("{} ({shortcut})", tab.label),
                None => tab.label,
            };
            let look = TabLook {
                tone: if tab.is_active {
                    TabTone::Active
                } else {
                    TabTone::Idle
                },
                is_temporary: tab.is_temporary,
            };
            tabs = tabs.push(tab_view(
                tab.icon,
                hint,
                look,
                tab.on_press,
                &palette,
            ));
        }
        if let Some(more) = value.more {
            let look = TabLook {
                tone: if more.is_open {
                    TabTone::MenuOpen
                } else {
                    TabTone::Idle
                },
                is_temporary: false,
            };
            tabs = tabs.push(tab_view(
                glyph::dots(),
                more.label,
                look,
                Some(more.on_press),
                &palette,
            ));
        }

        let background = palette.surface_raised();
        container(tabs)
            .padding(GROUP_PADDING)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_MD.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

/// Which of the three looks a tab has, hover aside.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TabTone {
    /// Transparent, muted icon.
    Idle,
    /// The active mode: `surface-hover`, accent icon.
    Active,
    /// The "⋯" button while its menu is open: `surface-hover`, strong icon.
    MenuOpen,
}

/// Look of one tab.
#[derive(Clone, Copy)]
struct TabLook {
    tone: TabTone,
    is_temporary: bool,
}

/// Builds one tab button with its tooltip.
fn tab_view<'a, Message: Clone + 'a>(
    icon: svg::Handle,
    hint: String,
    look: TabLook,
    on_press: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let idle = tab_icon_color(palette, look.tone);
    let hover = palette.text_strong();
    let icon =
        svg(icon)
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(move |_, status| svg::Style {
                color: Some(match status {
                    svg::Status::Hovered => hover,
                    _ => idle,
                }),
            });

    let colors = palette.clone();
    let control = button(
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
    )
    .width(size::MODE_TAB_WIDTH)
    .height(size::MODE_TAB_HEIGHT)
    .padding(0)
    .on_press_maybe(on_press)
    .style(move |_, _| tab_button_style(&colors, look));

    tooltip(
        control,
        icon_button::tooltip_hint(hint, palette),
        tooltip::Position::Bottom,
    )
    .gap(spacing::SPACE_1)
    .into()
}

/// Icon color of a tab while idle (not hovered).
fn tab_icon_color(palette: &UiPalette, tone: TabTone) -> Color {
    match tone {
        TabTone::Idle => palette.text_muted(),
        TabTone::Active => palette.accent(),
        TabTone::MenuOpen => palette.text_strong(),
    }
}

/// Button look: `surface-hover` when active or open, a `line` outline when
/// temporary.
fn tab_button_style(palette: &UiPalette, look: TabLook) -> button::Style {
    let border = Border {
        color: palette.line(),
        width: if look.is_temporary {
            OUTLINE_WIDTH
        } else {
            0.0
        },
        radius: radius::RADIUS_SM.into(),
    };

    button::Style {
        background: (look.tone != TabTone::Idle)
            .then(|| palette.surface_hover().into()),
        border,
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Terminal,
        Docker,
        More,
    }

    fn switcher(palette: &UiPalette) -> ModeSwitcher<Message> {
        ModeSwitcher::new(palette)
            .tab(
                ModeTab::new(glyph::plus(), "Terminal")
                    .active(true)
                    .on_press(Message::Terminal),
            )
            .tab(
                ModeTab::new(glyph::plus(), "Docker")
                    .shortcut("Ctrl Shift 2")
                    .temporary(true)
                    .on_press(Message::Docker),
            )
            .more("All modes", false, Message::More)
    }

    /// Center of the tab at `index` when the switcher sits at the origin.
    fn tab_center(index: usize) -> Point {
        Point::new(
            GROUP_PADDING
                + size::MODE_TAB_WIDTH / 2.0
                + index as f32 * (size::MODE_TAB_WIDTH + TAB_GAP),
            GROUP_PADDING + size::MODE_TAB_HEIGHT / 2.0,
        )
    }

    fn click_at(ui: &mut Simulator<'_, Message>, point: Point) {
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());
    }

    fn look(tone: TabTone, is_temporary: bool) -> TabLook {
        TabLook { tone, is_temporary }
    }

    #[test]
    fn given_switcher_when_tab_pressed_then_its_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(switcher(&palette));

        click_at(&mut ui, tab_center(1));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Docker]);
    }

    #[test]
    fn given_switcher_when_active_tab_pressed_then_its_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(switcher(&palette));

        click_at(&mut ui, tab_center(0));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Terminal]);
    }

    #[test]
    fn given_switcher_when_more_pressed_then_more_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(switcher(&palette));

        click_at(&mut ui, tab_center(2));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::More]);
    }

    #[test]
    fn given_tab_without_message_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            ModeSwitcher::<Message>::new(&palette)
                .tab(ModeTab::new(glyph::plus(), "Terminal")),
        );

        click_at(&mut ui, tab_center(0));

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_active_tab_then_it_is_hover_surface_with_accent_icon() {
        let palette = UiPalette::otty_dark();

        let style = tab_button_style(&palette, look(TabTone::Active, false));

        assert_eq!(style.background, Some(palette.surface_hover().into()));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(tab_icon_color(&palette, TabTone::Active), palette.accent());
    }

    #[test]
    fn given_idle_tab_then_it_has_no_background_and_a_muted_icon() {
        let palette = UiPalette::otty_dark();

        let style = tab_button_style(&palette, look(TabTone::Idle, false));

        assert_eq!(style.background, None);
        assert_eq!(
            tab_icon_color(&palette, TabTone::Idle),
            palette.text_muted()
        );
    }

    #[test]
    fn given_open_more_button_then_it_is_hover_surface_with_strong_icon() {
        let palette = UiPalette::otty_light();

        let style = tab_button_style(&palette, look(TabTone::MenuOpen, false));

        assert_eq!(style.background, Some(palette.surface_hover().into()));
        assert_eq!(
            tab_icon_color(&palette, TabTone::MenuOpen),
            palette.text_strong()
        );
    }

    #[test]
    fn given_temporary_tab_then_it_has_a_one_pixel_line_outline() {
        let palette = UiPalette::otty_light();

        let style = tab_button_style(&palette, look(TabTone::Idle, true));

        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.line());
    }
}
