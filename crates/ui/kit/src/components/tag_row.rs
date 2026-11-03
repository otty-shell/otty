//! `TagRow`: the tags of a [`TagSearch`] as a wrapping row of chips.
//!
//! [`TagSearch`]: super::tag_search::TagSearch

use iced::widget::{Space, row};
use iced::{Alignment, Element};

use super::button::{Button, ButtonKind, ButtonSize};
use super::chip::Chip;
use super::tag_search::{Tag, TagKey, TagSearchAction};
use crate::palette::UiPalette;
use crate::spacing;

/// A wrapping row of chips, one per tag, each with a clear cross, and a small
/// "Clear all" button at the end. With no tags it draws nothing and takes no
/// height. Without an action handler the chips have no cross and the button
/// is disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tag_row::TagRow;
/// use otty_ui_kit::components::tag_search::{Tag, TagKey, TagSearchAction};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Search(TagSearchAction),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let tags = [Tag::new("status", "Running")];
/// let keys = [TagKey::new("status", "Status")];
/// let row: Element<'_, Message> = TagRow::new(&tags, &keys, &palette)
///     .remove_label("Remove this tag")
///     .clear_all_label("Clear all")
///     .on_action(Message::Search)
///     .into();
/// ```
pub struct TagRow<'a, Message> {
    labels: Vec<String>,
    palette: UiPalette,
    remove_label: String,
    clear_all_label: String,
    on_action: Option<Box<dyn Fn(TagSearchAction) -> Message + 'a>>,
}

impl<'a, Message> TagRow<'a, Message> {
    /// Creates a row that shows `tags`, each with the shown name of its key
    /// in `keys`.
    pub fn new(tags: &[Tag], keys: &[TagKey], palette: &UiPalette) -> Self {
        Self {
            labels: tags.iter().map(|tag| tag.label(keys)).collect(),
            palette: palette.clone(),
            remove_label: String::new(),
            clear_all_label: String::new(),
            on_action: None,
        }
    }

    /// Sets the tooltip of the cross of each chip.
    pub fn remove_label(mut self, label: impl Into<String>) -> Self {
        self.remove_label = label.into();
        self
    }

    /// Sets the text of the button at the end of the row.
    pub fn clear_all_label(mut self, label: impl Into<String>) -> Self {
        self.clear_all_label = label.into();
        self
    }

    /// Sets the function that maps a [`TagSearchAction`] to the app message.
    pub fn on_action(
        mut self,
        on_action: impl Fn(TagSearchAction) -> Message + 'a,
    ) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }
}

impl<'a, Message: Clone + 'a> From<TagRow<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TagRow<'a, Message>) -> Self {
        if value.labels.is_empty() {
            return Space::new().into();
        }

        let palette = value.palette;
        let send = |action: TagSearchAction| {
            value.on_action.as_ref().map(|on_action| on_action(action))
        };

        let mut line =
            row![].spacing(spacing::SPACE_2).align_y(Alignment::Center);
        for (index, label) in value.labels.iter().enumerate() {
            let mut chip = Chip::new(label.as_str(), &palette).mono(true);
            if let Some(message) = send(TagSearchAction::RemoveTag(index)) {
                chip = chip.on_clear(value.remove_label.as_str(), message);
            }
            line = line.push(chip);
        }

        let mut clear_all = Button::new(value.clear_all_label, &palette)
            .kind(ButtonKind::Ghost)
            .size(ButtonSize::Small);
        if let Some(message) = send(TagSearchAction::ClearTags) {
            clear_all = clear_all.on_press(message);
        }

        line.push(clear_all)
            .wrap()
            .vertical_spacing(spacing::SPACE_2)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::widget::{column, container, text};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::spacing;

    const CLEAR_ALL: &str = "Clear all";

    fn keys() -> Vec<TagKey> {
        vec![
            TagKey::new("status", "Status"),
            TagKey::new("compose", "Compose"),
            TagKey::new("image", "Image"),
        ]
    }

    fn tags() -> Vec<Tag> {
        vec![
            Tag::new("status", "Running"),
            Tag::new("compose", "otty-site"),
            Tag::new("image", "busybox"),
        ]
    }

    fn view<'a>(tags: &'a [Tag], width: f32) -> Element<'a, TagSearchAction> {
        view_with(tags, &keys(), width)
    }

    fn view_with<'a>(
        tags: &'a [Tag],
        keys: &[TagKey],
        width: f32,
    ) -> Element<'a, TagSearchAction> {
        let palette = UiPalette::otty_dark();

        container(column![
            TagRow::new(tags, keys, &palette)
                .remove_label("Remove")
                .clear_all_label(CLEAR_ALL)
                .on_action(|action| action),
            text("below"),
        ])
        .width(width)
        .into()
    }

    /// Clicks the cross of the chip showing `label`: it sits one gap right
    /// of the label and is 24px wide.
    fn click_cross(ui: &mut Simulator<'_, TagSearchAction>, label: &str) {
        let bounds = ui.find(label).expect("chip should show").bounds();

        ui.point_at(Point::new(
            bounds.x + bounds.width + spacing::SPACE_2 + 12.0,
            bounds.y + bounds.height / 2.0,
        ));
        let _ = ui.simulate(simulator::click());
    }

    #[test]
    fn no_tags_draw_nothing_and_take_no_height() {
        let mut ui = simulator(view(&[], 400.0));

        let below = ui.find("below").expect("marker should show").bounds();

        assert_eq!(below.y, 0.0);
        assert!(ui.find(CLEAR_ALL).is_err());
    }

    #[test]
    fn each_tag_reads_key_colon_value_and_clear_all_comes_last() {
        let tags = tags();
        let mut ui = simulator(view(&tags, 900.0));

        let first = ui.find("Status:Running").expect("chip").bounds();
        let last = ui.find("Image:busybox").expect("chip").bounds();
        let clear = ui.find(CLEAR_ALL).expect("button").bounds();

        assert!(first.x < last.x);
        assert!(last.x < clear.x);
    }

    #[test]
    fn a_tag_shows_the_shown_name_of_its_key_and_never_the_internal_name() {
        let tags = tags();
        let russian = vec![
            TagKey::new("status", "Состояние"),
            TagKey::new("compose", "Compose"),
            TagKey::new("image", "Образ"),
        ];
        let mut ui = simulator(view_with(&tags, &russian, 900.0));

        assert!(ui.find("Состояние:Running").is_ok());
        assert!(ui.find("Образ:busybox").is_ok());
        assert!(ui.find("Status:Running").is_err());
        assert!(ui.find("status:Running").is_err());
    }

    #[test]
    fn chips_wrap_at_a_narrow_width() {
        let tags = tags();
        let mut ui = simulator(view(&tags, 200.0));

        let first = ui.find("Status:Running").expect("chip").bounds();
        let second = ui.find("Compose:otty-site").expect("chip").bounds();
        let below = ui.find("below").expect("marker").bounds();

        assert!(second.y > first.y, "second chip on a new line");
        assert!(below.y >= second.y + second.height);
    }

    #[test]
    fn a_cross_removes_its_tag() {
        let tags = tags();
        let mut ui = simulator(view(&tags, 900.0));

        click_cross(&mut ui, "Compose:otty-site");

        let messages: Vec<TagSearchAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![TagSearchAction::RemoveTag(1)]);
    }

    #[test]
    fn clear_all_clears_the_tags() {
        let tags = tags();
        let mut ui = simulator(view(&tags, 900.0));

        ui.click(CLEAR_ALL).expect("button should show");

        let messages: Vec<TagSearchAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![TagSearchAction::ClearTags]);
    }

    #[test]
    fn without_a_handler_nothing_is_sent() {
        let tags = tags();
        let palette = UiPalette::otty_dark();
        let row: Element<'_, TagSearchAction> =
            TagRow::new(&tags, &keys(), &palette)
                .clear_all_label(CLEAR_ALL)
                .into();
        let mut ui = simulator(row);

        ui.click(CLEAR_ALL).expect("button should show");
        click_cross(&mut ui, "Status:Running");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
