//! The notifications list, opened by the bell in the header.

use iced::widget::{Space, container, mouse_area, opaque, stack, svg};
use iced::{Element, Length, Padding, alignment, mouse};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::notification_list::NotificationList;
use otty_ui_kit::spacing;

use crate::event::NotificationsIntent;
use crate::model::NotificationListViewModel;

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
/// under the bell, over a transparent layer. The broom empties it, the
/// cross of a row removes that row, and a press of any button outside it
/// closes it.
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
        svg::Handle::from_memory(icons::BROOM),
        i18n::t(Key::NotificationsClear),
        vm.can_clear.then_some(NotificationsIntent::Clear),
    )
    .remove_label(i18n::t(Key::NotificationRemove))
    .empty(i18n::t(Key::NotificationsEmpty));
    for row in vm.rows {
        list = list.row(row.text, row.age, NotificationsIntent::Remove(row.id));
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
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_app_testing::mouse::{middle_click, right_click};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::NotificationListRowViewModel;

    /// Horizontal middle of the broom and of the row crosses of the open
    /// list in the 1024 px wide test window. The list ends 8 px before the
    /// right edge of the window; inside it, the 24 px wide controls end 9 px
    /// before its right edge.
    const CONTROLS_X: f32 = 1024.0 - 8.0 - 9.0 - 12.0;

    fn list(
        rows: &[(u64, &'static str, &str)],
    ) -> NotificationListViewModel<'static> {
        NotificationListViewModel {
            rows: rows
                .iter()
                .map(|(id, text, age)| NotificationListRowViewModel {
                    id: *id,
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
        let _ = drive(list(&[(0, "disk full", "just now")]), |ui| {
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

    /// Middle of the cross of the row that shows `text`, with `age` under
    /// it. The cross is centered on the text and the age, so the age is
    /// looked for right under the text: other rows can have the same age.
    fn notification_row_cross(
        ui: &mut Simulator<'_, NotificationsIntent>,
        text: &str,
        age: &str,
    ) -> Point {
        let text = ui.find(text).expect("the row should show").bounds();
        let below_text = text.y + text.height;
        let age = ui
            .find(|candidate: Candidate<'_>| match candidate {
                Candidate::Text {
                    content, bounds, ..
                } if content == age
                    && bounds.y >= below_text
                    && bounds.y < below_text + 8.0 =>
                {
                    Some(bounds)
                },
                _ => None,
            })
            .expect("the age should show under the text");

        Point::new(CONTROLS_X, (text.y + age.y + age.height) / 2.0)
    }

    /// Middle of the broom of the open list, which is centered on the title
    /// line.
    fn notification_list_broom(
        ui: &mut Simulator<'_, NotificationsIntent>,
    ) -> Point {
        let title = ui
            .find(i18n::t(Key::NotificationsTitle))
            .expect("the list should show")
            .bounds();

        Point::new(CONTROLS_X, title.center_y())
    }

    fn click_at(ui: &mut Simulator<'_, NotificationsIntent>, spot: Point) {
        ui.point_at(spot);
        let _ = ui.simulate(iced_test::simulator::click());
    }

    #[test]
    fn the_broom_empties_the_list() {
        let intents = drive(list(&[(0, "disk full", "just now")]), |ui| {
            let broom = notification_list_broom(ui);
            click_at(ui, broom);
        });

        assert!(matches!(intents.as_slice(), [NotificationsIntent::Clear]));
    }

    #[test]
    fn the_broom_does_nothing_with_nothing_to_clear() {
        let intents = drive(list(&[]), |ui| {
            let broom = notification_list_broom(ui);
            click_at(ui, broom);
        });

        assert!(intents.is_empty());
    }

    /// Owner: each row has its own cross, which removes only that row.
    #[test]
    fn a_row_cross_removes_only_that_row() {
        let rows = [(7, "disk full", "just now"), (3, "busy", "just now")];

        let intents = drive(list(&rows), |ui| {
            let cross = notification_row_cross(ui, "busy", "just now");
            click_at(ui, cross);
        });

        assert!(matches!(
            intents.as_slice(),
            [NotificationsIntent::Remove(3)]
        ));
    }

    #[test]
    fn a_press_on_a_row_keeps_the_list_open() {
        let intents = drive(list(&[(0, "disk full", "just now")]), |ui| {
            ui.click("disk full").expect("row should show");
        });

        assert!(intents.is_empty());
    }

    #[test]
    fn a_click_outside_the_list_closes_it() {
        let intents = drive(list(&[(0, "disk full", "just now")]), |ui| {
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
            let _ = ui.simulate(right_click());
        });

        assert!(matches!(
            middle.as_slice(),
            [NotificationsIntent::CloseList]
        ));
        assert!(matches!(right.as_slice(), [NotificationsIntent::CloseList]));
    }
}
