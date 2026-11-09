use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::explorer::event::ExplorerIntent;

/// Deliver a Files panel intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: ExplorerIntent) -> Task<AppEvent> {
    app.widgets.explorer.reduce(intent).map(AppEvent::Explorer)
}
