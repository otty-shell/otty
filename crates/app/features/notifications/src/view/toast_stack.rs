//! The toasts: errors at the top center of the window, newest on top.
//! They are no overlay: Esc leaves them, they take no keyboard focus, and
//! a dialog's scrim lies over them.

use iced::widget::{column, container, opaque, svg};
use iced::{Element, Length, Padding, alignment};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::spacing;

use crate::event::NotificationsIntent;
use crate::model::ToastStackViewModel;

/// Distance between the top of the main area and the first toast, as for
/// the focus mode note.
const TOAST_TOP_GAP: f32 = 12.0;
/// Width of a toast.
const TOAST_WIDTH: f32 = 420.0;

/// Props for the toast stack.
pub(crate) struct ToastStackProps<'a> {
    pub(crate) vm: ToastStackViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the toasts at the top center of the main area, newest on top,
/// each a floating kit `Notice` whose cross hides it. Return `None` while
/// no toast shows.
///
/// The returned layer is full size, but only the toasts take presses, so
/// presses beside them still reach the layers below.
pub(crate) fn view(
    props: ToastStackProps<'_>,
) -> Option<Element<'_, NotificationsIntent>> {
    if props.vm.toasts.is_empty() {
        return None;
    }

    let palette = props.theme.theme.ui();
    let mut toasts = column![].spacing(spacing::SPACE_2).width(TOAST_WIDTH);
    for toast in props.vm.toasts {
        let notice = Notice::new(
            toast.text,
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        )
        .floating()
        .on_dismiss(
            NotificationsIntent::DismissToast(toast.id),
            i18n::t(Key::NotificationDismiss),
        );
        // A press on a toast must not reach the panes under it.
        toasts = toasts.push(opaque(notice));
    }

    Some(
        container(toasts)
            .padding(Padding {
                top: TOAST_TOP_GAP,
                ..Padding::ZERO
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Top)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::ToastStackItemViewModel;

    /// Width of the simulator window.
    const WINDOW_WIDTH: f32 = 1024.0;

    fn stack(toasts: &[(u64, &str)]) -> ToastStackViewModel {
        ToastStackViewModel {
            toasts: toasts
                .iter()
                .map(|(id, text)| ToastStackItemViewModel {
                    id: *id,
                    text: String::from(*text),
                })
                .collect(),
        }
    }

    fn drive(
        toasts: &[(u64, &str)],
        steps: impl FnOnce(&mut Simulator<'_, NotificationsIntent>),
    ) -> Vec<NotificationsIntent> {
        let themes = ThemeManager::new();
        let element = view(ToastStackProps {
            vm: stack(toasts),
            theme: ThemeProps::new(themes.current()),
        })
        .expect("the toasts should show");
        let mut ui = simulator(element);

        steps(&mut ui);

        ui.into_messages().collect()
    }

    #[test]
    fn no_toast_draws_nothing() {
        let themes = ThemeManager::new();

        let element = view(ToastStackProps {
            vm: stack(&[]),
            theme: ThemeProps::new(themes.current()),
        });

        assert!(element.is_none());
    }

    #[test]
    fn the_toasts_lie_at_the_top_center_newest_on_top() {
        let _ = drive(&[(2, "busy"), (1, "disk full")], |ui| {
            let newest = ui.find("busy").expect("newest toast").bounds();
            let older = ui.find("disk full").expect("older toast").bounds();

            assert!(newest.y < older.y);
            assert!(newest.y < 40.0, "{newest:?}");
            assert!(newest.x > WINDOW_WIDTH / 2.0 - 220.0, "{newest:?}");
            assert!(newest.x < WINDOW_WIDTH / 2.0, "{newest:?}");
        });
    }

    #[test]
    fn the_cross_of_a_toast_hides_that_toast() {
        let intents = drive(&[(2, "busy"), (1, "disk full")], |ui| {
            let older = ui.find("disk full").expect("older toast").bounds();
            // The cross ends the toast: 12 px padding, then 16 px wide,
            // and the toast is 420 px wide in the middle of the window.
            let right = WINDOW_WIDTH / 2.0 + 210.0;
            ui.point_at(Point::new(right - 12.0 - 8.0, older.center_y()));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(matches!(
            intents.as_slice(),
            [NotificationsIntent::DismissToast(1)]
        ));
    }

    #[test]
    fn a_press_on_the_text_of_a_toast_sends_nothing() {
        let intents = drive(&[(1, "disk full")], |ui| {
            ui.click("disk full").expect("toast should show");
        });

        assert!(intents.is_empty());
    }
}
