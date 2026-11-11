//! The one delivery of the app: a component reduces an intent once, and
//! every outcome it reports is read in the same update.

use iced::Task;
use otty_app_slots::Component;

use crate::app::App;
use crate::message::AppEvent;

/// How a component is wired into the app: where it lives in `App`, what it
/// is given, and the one call each of its outcomes makes.
///
/// It is implemented on the component's intent type, so a call names only
/// the intent and the component follows from it:
/// `deliver(app, TabsIntent::CloseActive)`.
pub(crate) trait Delivery: Sized {
    /// The component that reduces this intent.
    type Component: Component<Intent = Self>;

    /// The component and the ctx it gets, taken from the fields of `App`.
    fn parts(
        app: &mut App,
    ) -> (
        &mut Self::Component,
        <Self::Component as Component>::Ctx<'_>,
    );

    /// One call for one outcome.
    fn read(
        app: &mut App,
        outcome: <Self::Component as Component>::Outcome,
    ) -> Task<AppEvent>;
}

/// The only delivery in the app: the component reduces the intent once,
/// then every outcome is read, in order, in the same update.
///
/// ```ignore
/// let task = deliver(app, TabsIntent::CloseActive);
/// ```
pub(crate) fn deliver<I: Delivery>(app: &mut App, intent: I) -> Task<AppEvent> {
    let (component, ctx) = I::parts(app);
    let outcomes = component.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| I::read(app, outcome))
        .collect();

    Task::batch(tasks)
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;
    use otty_app_domain::quick_launch::WizardTabInit;
    use otty_app_modes::ModesIntent;
    use otty_app_quick_launch::QuickLaunchScreen;
    use otty_app_slots::{Component, MainArea, MainContent};
    use otty_app_tabs::{SESSION_LIST_SCROLL_ID, TabsIntent};
    use otty_app_theme::theme::ThemeProps;

    use super::*;
    use crate::testing::task_outputs::snaps_to_end;
    use crate::testing::window::app;

    /// The tabs report three things when the launch editor opens: the
    /// editor tab opened, the sessions list scrolls to it and a tab came to
    /// the front. Every outcome is read in the same update: quick launch
    /// fills the editor, the returned task scrolls the list and the modes
    /// switch to Terminal mode.
    #[test]
    fn every_outcome_of_one_intent_is_read_in_the_same_update() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        let task = deliver(
            &mut app,
            TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: Vec::new(),
                },
            },
        );

        let tab_id = app.tabs.active_tab_id().expect("the editor tab");
        let theme = app.theme_manager.current().clone();
        let shown = match app.quick_launch.main_area(ThemeProps::new(&theme)) {
            MainContent::Screen { id, .. } => Some(id),
            MainContent::ScrollsOnItsOwn(_) => None,
        };
        assert_eq!(shown, Some(QuickLaunchScreen::Editor(tab_id)));
        assert!(snaps_to_end(task, &SESSION_LIST_SCROLL_ID.into()));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }
}
