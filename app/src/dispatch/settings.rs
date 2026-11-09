use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::settings::event::SettingsIntent;

/// Deliver a settings intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: SettingsIntent) -> Task<AppEvent> {
    app.widgets.settings.reduce(intent).map(AppEvent::Settings)
}
