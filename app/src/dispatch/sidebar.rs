use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::sidebar::event::SidebarIntent;

/// Deliver a sidebar intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: SidebarIntent) -> Task<AppEvent> {
    app.widgets.sidebar.reduce(intent).map(AppEvent::Sidebar)
}
