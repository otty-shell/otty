use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::settings::event::SettingsIntent;

/// Deliver a settings intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: SettingsIntent) -> Task<AppEvent> {
    app.widgets.settings.reduce(intent).map(AppEvent::Settings)
}
