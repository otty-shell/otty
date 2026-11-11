//! `TagRow`: the tags of a [`TagSearch`] as a wrapping row of chips.
//!
//! [`TagSearch`]: super::tag_search::TagSearch

use iced::widget::{Space, responsive, row};
use iced::{Alignment, Element, Length};

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

        // A chip's preferred width is its whole text, so the row's width is
        // handed to each chip, which wraps its text to fit. `responsive`
        // fills the height by default; shrinking keeps the row as tall as
        // its lines.
        responsive(move |available| {
            let palette = &value.palette;
            let send = |action: TagSearchAction| {
                value.on_action.as_ref().map(|on_action| on_action(action))
            };

            let mut line =
                row![].spacing(spacing::SPACE_2).align_y(Alignment::Center);
            for (index, label) in value.labels.iter().enumerate() {
                let mut chip = Chip::new(label.as_str(), palette)
                    .mono(true)
                    .wrap_within(available.width);
                if let Some(message) = send(TagSearchAction::RemoveTag(index)) {
                    chip = chip.on_clear(value.remove_label.as_str(), message);
                }
                line = line.push(chip);
            }

            let mut clear_all =
                Button::new(value.clear_all_label.as_str(), palette)
                    .kind(ButtonKind::Ghost)
                    .size(ButtonSize::Small);
            if let Some(message) = send(TagSearchAction::ClearTags) {
                clear_all = clear_all.on_press(message);
            }

            line.push(clear_all)
                .wrap()
                .vertical_spacing(spacing::SPACE_2)
                .into()
        })
        .height(Length::Shrink)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::widget::{column, container, text};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::{size, spacing, typography};

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

    const LONG_VALUE: &str =
        "registry.example.com/team/very-long-image-name/with/many/parts:1.2.3";
    const LONG_LABEL: &str = "Image:registry.example.com/team/very-long-image-name/with/many/parts:1.2.3";

    fn long_tags() -> Vec<Tag> {
        vec![Tag::new("status", "Running"), Tag::new("image", LONG_VALUE)]
    }

    #[test]
    fn a_long_tag_wraps_and_its_chip_grows_inside_the_row() {
        let tags = long_tags();
        let mut ui = simulator(view(&tags, 200.0));

        let short = ui.find("Status:Running").expect("chip").bounds();
        let long = ui.find(LONG_LABEL).expect("long chip").bounds();
        let below = ui.find("below").expect("marker").bounds();

        assert!(
            long.height > typography::SMALL.line_height() * 1.5,
            "text should take several lines: {long:?}"
        );
        // The text, a gap, the 24px cross and the chip padding fit the row.
        let chip_right =
            long.x + long.width + spacing::SPACE_2 + 24.0 + spacing::SPACE_3;
        assert!(
            chip_right <= 200.5,
            "chip is wider than the row: {chip_right}"
        );
        assert!(long.x >= 0.0);
        assert!(
            below.y >= long.y + long.height,
            "the content below moves down"
        );
        assert!(
            (short.height - typography::SMALL.line_height()).abs() < 0.5,
            "a short tag stays one line: {short:?}"
        );
    }

    #[test]
    fn a_row_of_short_tags_is_one_line_as_tall_as_the_clear_all_button() {
        let tags = vec![Tag::new("status", "Running")];
        let mut ui = simulator(view(&tags, 900.0));

        let tag = ui.find("Status:Running").expect("chip").bounds();
        let below = ui.find("below").expect("marker").bounds();

        assert!(
            (below.y - size::CONTROL_MD).abs() < 0.5,
            "one line of {} px: {}",
            size::CONTROL_MD,
            below.y
        );
        assert!((tag.height - typography::SMALL.line_height()).abs() < 0.5);
    }

    #[test]
    fn the_cross_of_a_long_tag_works_and_does_not_overlap_the_text() {
        let tags = long_tags();
        let mut ui = simulator(view(&tags, 200.0));
        let text = ui.find(LONG_LABEL).expect("long chip").bounds();

        // The cross sits in the middle of the chip's height.
        ui.point_at(Point::new(
            text.x + text.width + spacing::SPACE_2 + 12.0,
            text.y + text.height / 2.0,
        ));
        let _ = ui.simulate(simulator::click());

        let messages: Vec<TagSearchAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![TagSearchAction::RemoveTag(1)]);
        assert!(text.x + text.width + spacing::SPACE_2 + 24.0 <= 200.5);
    }

    #[test]
    fn the_cross_of_a_long_tag_is_not_at_the_first_line() {
        let tags = long_tags();
        let mut ui = simulator(view(&tags, 200.0));
        let text = ui.find(LONG_LABEL).expect("long chip").bounds();
        assert!(text.height > 30.0, "chip should be tall: {text:?}");

        // Next to the first line of a tall chip there is no cross.
        ui.point_at(Point::new(
            text.x + text.width + spacing::SPACE_2 + 12.0,
            text.y + 1.0,
        ));
        let _ = ui.simulate(simulator::click());

        let messages: Vec<TagSearchAction> = ui.into_messages().collect();
        assert!(messages.is_empty());
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
