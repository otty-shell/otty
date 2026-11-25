//! `ModeSwitcher`: row of icon tabs that picks the active mode.

use iced::widget::{button, container, svg};
use iced::{Border, Color, Element, Length};

use super::glyph;
use super::icon_hover::IconHover;
use super::mode_dock::ModeDock;
use super::tooltip::tooltip;
use crate::palette::UiPalette;
use crate::{radius, size};

/// Padding of the group (`padding: 3px`).
const GROUP_PADDING: f32 = 3.0;
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
/// No tab and not the "⋯" button ever shrinks. When the tabs do not fit,
/// they scroll sideways under the "⋯" button, which stays at the right end,
/// and the active tab is scrolled into view when it changes.
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
        let active = value.tabs.iter().position(|tab| tab.is_active);

        let mut tabs = Vec::with_capacity(value.tabs.len());
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
            tabs.push(tab_view(tab.icon, hint, look, tab.on_press, &palette));
        }
        let more = value.more.map(|more| {
            let look = TabLook {
                tone: if more.is_open {
                    TabTone::MenuOpen
                } else {
                    TabTone::Idle
                },
                is_temporary: false,
            };

            tab_view(
                glyph::dots(),
                more.label,
                look,
                Some(more.on_press),
                &palette,
            )
        });

        let background = palette.surface_raised();
        container(ModeDock::new(tabs, more, active, background))
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
    let pointer = IconHover::new();
    let icon = svg(icon)
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(pointer.icon_style(idle, hover));

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

    tooltip(pointer.watch(control), hint, palette)
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
    use iced::{Event, Point, mouse};
    use iced_test::{Simulator, simulator};

    use super::super::mode_dock::TAB_GAP;
    use super::*;
    use crate::testing::live_app::LiveApp;
    use crate::testing::pictures::assert_icon_lights_up;

    #[test]
    fn given_pointer_on_tab_corner_then_icon_is_lit_like_over_the_icon() {
        let view = || {
            let look = TabLook {
                tone: TabTone::Idle,
                is_temporary: false,
            };

            tab_view(
                glyph::plus(),
                "Terminal".to_owned(),
                look,
                Some(()),
                &UiPalette::otty_dark(),
            )
        };

        assert_icon_lights_up(
            "mode-tab",
            view,
            iced::Rectangle::new(
                iced::Point::ORIGIN,
                iced::Size::new(size::MODE_TAB_WIDTH, size::MODE_TAB_HEIGHT),
            ),
        );
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Terminal,
        Containers,
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
                ModeTab::new(glyph::plus(), "Containers")
                    .shortcut("Ctrl Shift 2")
                    .temporary(true)
                    .on_press(Message::Containers),
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

    fn click_at<M>(ui: &mut Simulator<'_, M>, point: Point) {
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
        assert_eq!(messages, vec![Message::Containers]);
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

    /// Tabs of the narrow dock tests.
    const MANY_TABS: usize = 5;
    /// A dock with room for two tabs and a part of the third left of More:
    /// the tabs take 82px of its 114px inside, More and its gap the rest.
    const NARROW: f32 = 120.0;
    /// Middle of the tabs and of More, from the top of the dock.
    const TABS_MIDDLE: f32 = GROUP_PADDING + size::MODE_TAB_HEIGHT / 2.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Pick {
        Tab(usize),
        More,
    }

    /// A dock of [`MANY_TABS`] tabs with More, `width` wide, with the tab at
    /// `active` active.
    fn many_tabs(width: f32, active: usize) -> Element<'static, Pick> {
        let palette = UiPalette::otty_dark();
        let mut switcher = ModeSwitcher::new(&palette);
        for index in 0..MANY_TABS {
            switcher = switcher.tab(
                ModeTab::new(glyph::plus(), format!("Mode {index}"))
                    .active(index == active)
                    .on_press(Pick::Tab(index)),
            );
        }
        let switcher = switcher.more("All modes", false, Pick::More);

        container(switcher).width(width).into()
    }

    /// What a press at `x` along the middle of the tabs sends, in a new
    /// dock `width` wide after `scroll` moved it over the tabs.
    fn pick_after(
        width: f32,
        scroll: Option<mouse::ScrollDelta>,
        x: f32,
    ) -> Vec<Pick> {
        let mut ui = simulator(many_tabs(width, 0));
        if let Some(delta) = scroll {
            ui.point_at(Point::new(tab_left(0) + 10.0, TABS_MIDDLE));
            let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
                delta,
            })]);
        }

        click_at(&mut ui, Point::new(x, TABS_MIDDLE));

        ui.into_messages().collect()
    }

    /// Left edge of the tab at `index` in a dock that is not scrolled.
    fn tab_left(index: usize) -> f32 {
        GROUP_PADDING + index as f32 * (size::MODE_TAB_WIDTH + TAB_GAP)
    }

    /// Left edge of More in a dock `width` wide.
    fn more_left(width: f32) -> f32 {
        width - GROUP_PADDING - size::MODE_TAB_WIDTH
    }

    #[test]
    fn given_a_narrow_dock_then_every_tab_keeps_its_width() {
        let last_pixel_of_first = tab_left(0) + size::MODE_TAB_WIDTH - 1.0;
        let gap = tab_left(0) + size::MODE_TAB_WIDTH + 1.0;
        let first_pixel_of_second = tab_left(1) + 1.0;

        assert_eq!(
            pick_after(NARROW, None, last_pixel_of_first),
            vec![Pick::Tab(0)]
        );
        assert_eq!(pick_after(NARROW, None, gap), vec![]);
        assert_eq!(
            pick_after(NARROW, None, first_pixel_of_second),
            vec![Pick::Tab(1)]
        );
    }

    #[test]
    fn given_any_room_then_more_keeps_its_width_at_the_right_end_over_the_tabs()
    {
        for width in [NARROW, 150.0] {
            let more = more_left(width);

            assert_eq!(
                pick_after(width, None, more + 1.0),
                vec![Pick::More],
                "{width}"
            );
            assert_eq!(
                pick_after(width, None, more + size::MODE_TAB_WIDTH - 1.0),
                vec![Pick::More],
                "{width}"
            );
            // A tab lies under the gap before More, which covers it.
            assert_eq!(pick_after(width, None, more - 1.0), vec![], "{width}");
        }
    }

    #[test]
    fn given_a_narrow_dock_when_the_wheel_turns_then_the_tabs_move_sideways() {
        let first_slot = tab_left(0) + size::MODE_TAB_WIDTH / 2.0;
        let notch_down = mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 };
        let notch_up = mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 };
        let swipe = mouse::ScrollDelta::Pixels {
            x: -(size::MODE_TAB_WIDTH + TAB_GAP),
            y: 0.0,
        };

        assert_eq!(
            pick_after(NARROW, Some(notch_down), first_slot),
            vec![Pick::Tab(1)]
        );
        assert_eq!(
            pick_after(NARROW, Some(swipe), first_slot),
            vec![Pick::Tab(1)]
        );
        assert_eq!(
            pick_after(NARROW, Some(notch_up), first_slot),
            vec![Pick::Tab(0)]
        );
    }

    #[test]
    fn given_a_narrow_dock_scrolled_to_the_end_then_the_last_tab_is_out_from_under_more()
     {
        let far_right = mouse::ScrollDelta::Pixels {
            x: -10_000.0,
            y: 0.0,
        };
        let more = more_left(NARROW);
        let last_tab_left = more - TAB_GAP - size::MODE_TAB_WIDTH;

        assert_eq!(
            pick_after(NARROW, Some(far_right), last_tab_left + 1.0),
            vec![Pick::Tab(MANY_TABS - 1)]
        );
        assert_eq!(
            pick_after(NARROW, Some(far_right), more - TAB_GAP - 1.0),
            vec![Pick::Tab(MANY_TABS - 1)]
        );
        assert_eq!(
            pick_after(NARROW, Some(far_right), more + 1.0),
            vec![Pick::More]
        );
    }

    #[test]
    fn given_a_wide_dock_when_the_wheel_turns_then_nothing_moves() {
        let wide = 400.0;
        let notch_down = mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 };
        let second = tab_left(1) + size::MODE_TAB_WIDTH / 2.0;
        let more = tab_left(MANY_TABS) + 1.0;
        let past_more = tab_left(MANY_TABS) + size::MODE_TAB_WIDTH + 1.0;

        assert_eq!(
            pick_after(wide, Some(notch_down), second),
            vec![Pick::Tab(1)]
        );
        assert_eq!(pick_after(wide, None, more), vec![Pick::More]);
        assert_eq!(pick_after(wide, None, past_more), vec![]);
    }

    /// The dock of a running app: the tab pressed last is the active one.
    fn live_dock() -> LiveApp<usize, Pick> {
        LiveApp::new(
            0,
            |active| many_tabs(NARROW, *active),
            |active, pick| {
                if let Pick::Tab(index) = pick {
                    *active = index;
                }
            },
        )
    }

    /// The tab active after a press at `x` along the middle of the tabs.
    fn press_in_live(app: &mut LiveApp<usize, Pick>, x: f32) -> usize {
        app.point_at(Point::new(x, TABS_MIDDLE));
        app.click();

        *app.state()
    }

    #[test]
    fn given_a_mode_change_to_a_hidden_tab_then_the_dock_scrolls_it_into_view()
    {
        let mut app = live_dock();
        app.send([]);
        let last_tab_middle =
            more_left(NARROW) - TAB_GAP - size::MODE_TAB_WIDTH / 2.0;

        // A shortcut picks the last mode, whose tab lies under More.
        *app.state_mut() = MANY_TABS - 1;

        assert_eq!(press_in_live(&mut app, last_tab_middle), MANY_TABS - 1);
    }

    #[test]
    fn given_a_press_on_a_tab_cut_by_more_then_the_dock_shows_it_whole() {
        let mut app = live_dock();
        let cut_part_of_third = tab_left(2) + 3.0;
        let shown_left = more_left(NARROW) - TAB_GAP - size::MODE_TAB_WIDTH;

        assert_eq!(press_in_live(&mut app, cut_part_of_third), 2);

        assert_eq!(press_in_live(&mut app, shown_left + 1.0), 2);
        assert_eq!(press_in_live(&mut app, shown_left - TAB_GAP - 1.0), 1);
    }
}
