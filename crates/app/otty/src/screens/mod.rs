//! The screen of each mode: which feature fills the left panel, the main
//! area, the right panel, the status bar and the mode overlays. `view.rs`
//! picks one by the active mode.
//!
//! Each feature fills its places here, through the place traits of
//! `otty-app-slots`.

pub(crate) mod agents;
pub(crate) mod containers;
pub(crate) mod plugins;
pub(crate) mod start_card;
pub(crate) mod terminal;

use iced::widget::{container, mouse_area, responsive};
use iced::{Element, Length};
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_modes::ModesIntent;
use otty_app_slots::{Overlay, OverlayKind};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::{MenuPlace, context_menu};
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::size;

use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::frame::sidebar::types::{HeaderMenu, OpenHeaderMenu};
use crate::layout;
use crate::message::AppEvent;

/// The element of `overlay`, its messages turned into app events by
/// `to_event`. Every message of a menu is marked as sent from inside the
/// open menu ([`AppEvent::FromMenu`]), so the input checks let it reach the
/// menu without listing its items.
pub(crate) fn overlay_layer<'a, Message: 'a>(
    overlay: Overlay<'a, Message>,
    to_event: impl Fn(Message) -> AppEvent + 'a,
) -> Element<'a, AppEvent> {
    let kind = overlay.kind();
    let content = overlay.into_content().map(to_event);

    match kind {
        OverlayKind::Menu => content.map(AppEvent::from_menu),
        _ => content,
    }
}

/// Lay `overlay` out in the main area of a layer over the whole window:
/// under the header and over the status bar. It gets the size of the main
/// area, and the pointer positions it reports are in the main area too.
pub(crate) fn over_main_area(
    overlay: Element<'_, AppEvent>,
) -> Element<'_, AppEvent> {
    container(overlay)
        .padding(layout::main_area_padding())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The open menu of a section header, at the point where it was opened
/// and kept inside the main area: the New session and Launches menus of
/// Terminal mode, or the Add plugin menu of Plugins mode. A press away
/// from it closes it (see [`AppEvent::PressedAway`]).
pub(crate) fn header_menu<'a>(
    app: &'a App,
    open: OpenHeaderMenu,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let items = match open.menu() {
        HeaderMenu::Sessions => terminal::new_session_menu(app, theme),
        HeaderMenu::Launches => terminal::launches_menu(app, theme),
        HeaderMenu::AddPlugin => plugins::add_menu(app, theme),
    };

    let layer = Element::from(context_menu(
        items,
        MenuPlace::new(open.cursor(), app.screen_size),
    ))
    .map(AppEvent::from_menu);

    // The layer covers the main area, so it reports where the pointer is
    // in place of the main area: the next menu opens at the right spot.
    mouse_area(layer)
        .on_move(|position| {
            AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { position })
        })
        .into()
}

/// The status bar of Containers, Agents and Plugins mode: on the left the items
/// `left` gives, from the feature of the mode; on the right the sessions
/// button, which shows the count of `sessions` and switches to Terminal
/// mode, and the OTTY version. In a bar too narrow for all of them the left
/// side gives way first: its last items are left out, whole, and the right
/// side keeps its size. The width is known only during layout, so `left`
/// is asked at each layout.
pub(crate) fn bar_with_sessions<'a>(
    palette: &'a UiPalette,
    sessions: usize,
    left: impl Fn() -> Vec<StatusItem<AppEvent>> + 'a,
) -> Element<'a, AppEvent> {
    let bar = responsive(move |room| {
        StatusBar::new(palette)
            .right(StatusItem::button(
                i18n::status_sessions(sessions),
                AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
            ))
            .right(StatusItem::text(format!(
                "OTTY {}",
                env!("CARGO_PKG_VERSION")
            )))
            .left_that_fit(left(), room.width)
            .into()
    });

    container(bar).height(size::STATUSBAR).into()
}
