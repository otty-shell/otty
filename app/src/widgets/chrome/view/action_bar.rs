use iced::widget::{MouseArea, Space, Stack, container, row, svg};
use iced::{Element, Length, alignment};
use otty_ui_kit::components::icon_button::{IconButton, IconTone};
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::components::title_search::TitleSearch;
use otty_ui_kit::{size, spacing};

use super::super::event::ChromeIntent;
use crate::i18n::{self, Key};
use crate::icons::{
    FOCUS_MODE, PANEL_LEFT, PANEL_RIGHT, SEARCH, SETTINGS, WINDOW_CLOSE,
    WINDOW_FULLSCREEN, WINDOW_TRAY,
};
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Props for rendering the title bar.
#[derive(Debug, Clone)]
pub(crate) struct ActionBarProps<'a> {
    /// Title of the active tab, shown in the search button.
    pub(crate) title: &'a str,
    pub(crate) theme: ThemeProps<'a>,
    /// The left panel is shown; its toggle uses the accent color.
    pub(crate) left_open: bool,
    /// The right (files) panel is shown; its toggle uses the accent color.
    pub(crate) right_open: bool,
    /// Focus mode is on; its toggle uses the accent color.
    pub(crate) focus_mode: bool,
}

/// Render the draggable title bar: left panel toggle, title search and the
/// right-side toggles and window controls.
pub(crate) fn view<'a>(props: ActionBarProps<'a>) -> Element<'a, ChromeIntent> {
    let palette = props.theme.theme.ui();

    let left_toggle = toggle_button(
        PANEL_LEFT,
        tooltip_label(i18n::t(Key::WindowLeftPanel), AppShortcut::LeftPanel),
        props.left_open,
        ChromeIntent::ToggleLeftPanel,
        palette,
    );

    // Pressing the search button will open the command palette (ticket 17).
    let title_search = TitleSearch::new(
        props.title,
        AppShortcut::CommandPalette.label(),
        svg::Handle::from_memory(SEARCH),
        palette,
    );

    let right_controls = row![
        toggle_button(
            PANEL_RIGHT,
            tooltip_label(
                i18n::t(Key::WindowFilesPanel),
                AppShortcut::FilesPanel,
            ),
            props.right_open,
            ChromeIntent::ToggleFilesPanel,
            palette,
        ),
        toggle_button(
            FOCUS_MODE,
            tooltip_label(
                i18n::t(Key::WindowFocusMode),
                AppShortcut::FocusMode,
            ),
            props.focus_mode,
            ChromeIntent::ToggleFocusMode,
            palette,
        ),
        IconButton::new(
            svg::Handle::from_memory(SETTINGS),
            tooltip_label(
                i18n::t(Key::SettingsTabTitle),
                AppShortcut::Settings,
            ),
            palette,
        )
        .on_press(ChromeIntent::OpenSettings),
        Space::new().width(spacing::SPACE_2),
        IconButton::new(
            svg::Handle::from_memory(WINDOW_FULLSCREEN),
            i18n::t(Key::WindowToggleFullscreen),
            palette,
        )
        .on_press(ChromeIntent::ToggleFullScreen),
        IconButton::new(
            svg::Handle::from_memory(WINDOW_TRAY),
            i18n::t(Key::WindowMinimize),
            palette,
        )
        .on_press(ChromeIntent::MinimizeWindow),
        IconButton::new(
            svg::Handle::from_memory(WINDOW_CLOSE),
            i18n::t(Key::WindowClose),
            palette,
        )
        .on_press(ChromeIntent::CloseWindow),
    ]
    .spacing(spacing::SPACE_1)
    .align_y(alignment::Vertical::Center);

    let bar = row![
        left_toggle,
        container(title_search)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
        right_controls,
    ]
    .spacing(spacing::SPACE_1)
    .align_y(alignment::Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    let bar = container(bar)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .padding([0.0, spacing::SPACE_2]);

    // The drag surface sits under the bar, so empty space between the
    // buttons still drags the window.
    let drag_surface = MouseArea::new(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(ChromeIntent::StartWindowDrag)
    .on_double_click(ChromeIntent::ToggleFullScreen);

    let content = Stack::new().push(drag_surface).push(bar);

    surface(content, Surface::Chrome, palette)
        .height(Length::Fixed(size::TITLEBAR))
        .into()
}

/// Build a panel or mode toggle; it uses the accent color while `is_on`.
fn toggle_button<'a>(
    icon: &'static [u8],
    label: String,
    is_on: bool,
    intent: ChromeIntent,
    palette: &otty_ui_kit::palette::UiPalette,
) -> Element<'a, ChromeIntent> {
    let tone = if is_on {
        IconTone::Accent
    } else {
        IconTone::Default
    };

    IconButton::new(svg::Handle::from_memory(icon), label, palette)
        .tone(tone)
        .on_press(intent)
        .into()
}

/// Tooltip text: the action name followed by its shortcut in brackets.
fn tooltip_label(name: &str, shortcut: AppShortcut) -> String {
    format!("{name} ({})", shortcut.label())
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;
    use crate::theme::ThemeManager;

    const BAR_Y: f32 = size::TITLEBAR / 2.0;

    fn props(themes: &ThemeManager) -> ActionBarProps<'_> {
        ActionBarProps {
            title: "zsh  ~/otty",
            theme: ThemeProps::new(themes.current()),
            left_open: true,
            right_open: false,
            focus_mode: false,
        }
    }

    /// Click the title bar at `x` and return the intents it emitted.
    fn click_at(x: f32) -> Vec<ChromeIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(props(&themes)));

        ui.point_at(Point::new(x, BAR_Y));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn left_panel_button_emits_its_intent() {
        assert!(matches!(
            click_at(20.0).as_slice(),
            [ChromeIntent::ToggleLeftPanel]
        ));
    }

    #[test]
    fn files_button_emits_its_intent() {
        assert!(matches!(
            click_at(834.0).as_slice(),
            [ChromeIntent::ToggleFilesPanel]
        ));
    }

    #[test]
    fn focus_button_emits_its_intent() {
        assert!(matches!(
            click_at(866.0).as_slice(),
            [ChromeIntent::ToggleFocusMode]
        ));
    }

    #[test]
    fn settings_button_emits_its_intent() {
        assert!(matches!(
            click_at(898.0).as_slice(),
            [ChromeIntent::OpenSettings]
        ));
    }

    #[test]
    fn window_buttons_emit_their_intents() {
        assert!(matches!(
            click_at(940.0).as_slice(),
            [ChromeIntent::ToggleFullScreen]
        ));
        assert!(matches!(
            click_at(972.0).as_slice(),
            [ChromeIntent::MinimizeWindow]
        ));
        assert!(matches!(
            click_at(1004.0).as_slice(),
            [ChromeIntent::CloseWindow]
        ));
    }

    #[test]
    fn empty_title_bar_space_starts_a_window_drag() {
        assert!(matches!(
            click_at(160.0).as_slice(),
            [ChromeIntent::StartWindowDrag]
        ));
    }

    #[test]
    fn title_search_shows_the_title_and_only_lets_the_window_drag() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(props(&themes)));

        ui.click("zsh  ~/otty").expect("title should be visible");

        let intents: Vec<ChromeIntent> = ui.into_messages().collect();
        assert!(
            intents
                .iter()
                .all(|intent| matches!(intent, ChromeIntent::StartWindowDrag))
        );
    }

    #[test]
    fn tooltip_names_the_action_and_its_shortcut() {
        assert_eq!(
            tooltip_label("Files", AppShortcut::FilesPanel),
            format!("Files ({})", AppShortcut::FilesPanel.label())
        );
    }
}
