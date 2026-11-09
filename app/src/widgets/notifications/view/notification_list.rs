//! The notifications list, opened by the bell in the header.

use iced::widget::{Space, container, mouse_area, opaque, stack, svg};
use iced::{Element, Length, Padding, alignment, mouse};
use otty_ui_kit::components::notification_list::NotificationList;
use otty_ui_kit::spacing;

use super::super::event::NotificationsIntent;
use super::super::model::NotificationListViewModel;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Distance from the header line to the list.
const LIST_TOP: f32 = spacing::SPACE_1;
/// Distance from the right edge of the window to the list, lined up with
/// the header buttons.
const LIST_RIGHT: f32 = spacing::SPACE_3;

/// Props for the notifications list.
pub(crate) struct NotificationListProps<'a> {
    pub(crate) vm: NotificationListViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the kit `NotificationList` at the top right of the main area,
/// under the bell, over a transparent layer. Clear empties it, and a press
/// of any button outside it closes it.
pub(crate) fn view<'a>(
    props: NotificationListProps<'a>,
) -> Element<'a, NotificationsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut list = NotificationList::new(
        i18n::t(Key::NotificationsTitle),
        svg::Handle::from_memory(icons::NOTICE_ERROR),
        palette,
    )
    .clear(
        i18n::t(Key::NotificationsClear),
        vm.can_clear.then_some(NotificationsIntent::Clear),
    )
    .empty(i18n::t(Key::NotificationsEmpty));
    for row in vm.rows {
        list = list.row(row.text, row.age);
    }
    let list: Element<'a, NotificationsIntent> = list.into();

    // Presses on the list itself must not reach the layer below.
    let positioned = container(opaque(list))
        .padding(Padding {
            top: LIST_TOP,
            right: LIST_RIGHT,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer =
        mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(NotificationsIntent::CloseList)
            .on_right_press(NotificationsIntent::CloseList)
            .on_middle_press(NotificationsIntent::CloseList)
            .interaction(mouse::Interaction::Idle);

    stack![dismiss_layer, positioned]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::i18n::{self, Key};
    use crate::testing::mouse::middle_click;
    use crate::theme::ThemeManager;
    use crate::widgets::notifications::model::NotificationListRowViewModel;

    fn list(
        rows: &[(&'static str, &str)],
    ) -> NotificationListViewModel<'static> {
        NotificationListViewModel {
            rows: rows
                .iter()
                .map(|(text, age)| NotificationListRowViewModel {
                    text,
                    age: String::from(*age),
                })
                .collect(),
            can_clear: !rows.is_empty(),
        }
    }

    fn drive(
        vm: NotificationListViewModel<'static>,
        steps: impl FnOnce(&mut Simulator<'_, NotificationsIntent>),
    ) -> Vec<NotificationsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(NotificationListProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        steps(&mut ui);

        ui.into_messages().collect()
    }

    #[test]
    fn the_list_shows_its_title_rows_and_ages_at_the_top_right() {
        let _ = drive(list(&[("disk full", "just now")]), |ui| {
            let title = ui
                .find(i18n::t(Key::NotificationsTitle))
                .expect("title should show")
                .bounds();
            ui.find("disk full").expect("row should show");
            ui.find("just now").expect("age should show");

            assert!(title.y < 40.0, "{title:?}");
            assert!(title.x > 1024.0 / 2.0, "{title:?}");
        });
    }

    #[test]
    fn an_empty_list_says_so() {
        let _ = drive(list(&[]), |ui| {
            ui.find(i18n::t(Key::NotificationsEmpty))
                .expect("empty text should show");
        });
    }

    #[test]
    fn clear_empties_the_list() {
        let intents = drive(list(&[("disk full", "just now")]), |ui| {
            ui.click(i18n::t(Key::NotificationsClear))
                .expect("Clear should show");
        });

        assert!(matches!(intents.as_slice(), [NotificationsIntent::Clear]));
    }

    #[test]
    fn clear_does_nothing_with_nothing_to_clear() {
        let intents = drive(list(&[]), |ui| {
            ui.click(i18n::t(Key::NotificationsClear))
                .expect("Clear should show");
        });

        assert!(intents.is_empty());
    }

    #[test]
    fn a_press_on_a_row_keeps_the_list_open() {
        let intents = drive(list(&[("disk full", "just now")]), |ui| {
            ui.click("disk full").expect("row should show");
        });

        assert!(intents.is_empty());
    }

    #[test]
    fn a_click_outside_the_list_closes_it() {
        let intents = drive(list(&[("disk full", "just now")]), |ui| {
            ui.point_at(Point::new(200.0, 600.0));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(matches!(
            intents.as_slice(),
            [NotificationsIntent::CloseList]
        ));
    }

    /// A middle or right press away from the list closes it, as a left
    /// press does.
    #[test]
    fn a_middle_or_right_press_outside_the_list_closes_it() {
        let middle = drive(list(&[]), |ui| {
            ui.point_at(Point::new(200.0, 600.0));
            let _ = ui.simulate(middle_click());
        });
        let right = drive(list(&[]), |ui| {
            ui.point_at(Point::new(200.0, 600.0));
            let _ = ui.simulate([
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Right,
                )),
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Right,
                )),
            ]);
        });

        assert!(matches!(
            middle.as_slice(),
            [NotificationsIntent::CloseList]
        ));
        assert!(matches!(right.as_slice(), [NotificationsIntent::CloseList]));
    }
}
