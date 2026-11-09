use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::sidebar::event::SidebarIntent;

/// Deliver a sidebar intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: SidebarIntent) -> Task<AppEvent> {
    app.widgets.sidebar.reduce(intent).map(AppEvent::Sidebar)
}
