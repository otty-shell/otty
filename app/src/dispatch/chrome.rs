use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::chrome::event::ChromeIntent;

/// Deliver a chrome intent to the widget's reducer.
pub(crate) fn deliver(app: &mut App, intent: ChromeIntent) -> Task<AppEvent> {
    app.widgets.chrome.reduce(intent).map(AppEvent::Chrome)
}
