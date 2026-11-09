use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::chrome::event::ChromeIntent;

/// Deliver a chrome intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: ChromeIntent) -> Task<AppEvent> {
    app.widgets.chrome.reduce(intent).map(AppEvent::Chrome)
}
