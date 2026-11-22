//! What a part can access, as the page of a plugin and the install and
//! update dialogs show it (issue 35): the title "Can access" with its
//! muted line, then one row per group, its label in a column on the left
//! as the facts of the page, its places on the right one per line in the
//! monospace font. A part that asks for nothing says so in one muted line.
//! The tag of the kind of a part sits here too, as both show it.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, row, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tag::tag;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use crate::event::PluginsIntent;
use crate::model::permissions::{
    PermissionGroupViewModel, PlaceMark, PlaceViewModel,
};

/// Width of the label of a group, a little wider than the keys of the
/// facts so "Hidden terminal" fits on one line.
const LABEL_WIDTH: f32 = 112.0;
/// Text size of the groups, as the facts of the page (`font-size:
/// 12.5px`).
const TEXT_SIZE: f32 = 12.5;
/// Gap between the places of a group.
const PLACE_GAP: f32 = 3.0;
/// Height of the line the kind tag is centred in: the tag is 16 px high.
const TAG_LINE: f32 = 20.0;

/// "Can access" with its muted line over the groups, or "Asks for no
/// permissions" when there are none.
pub(super) fn can_access<'a>(
    groups: Vec<PermissionGroupViewModel>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    if groups.is_empty() {
        return small_text(
            i18n::t(Key::PluginsNoPermissions),
            palette.text_muted(),
        )
        .into();
    }

    let title = column![
        text(i18n::t(Key::PluginsCanAccess))
            .size(TEXT_SIZE)
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        small_text(i18n::t(Key::PluginsCanAccessNote), palette.text_muted()),
    ]
    .spacing(2.0);

    let mut rows = Column::new().spacing(spacing::SPACE_3);
    for group in groups {
        rows = rows.push(group_row(group, palette));
    }

    column![title, rows].spacing(spacing::SPACE_3).into()
}

/// The kind of a part, such as "Agent", as a small tag centred on the line
/// of the part's name.
pub(super) fn kind_tag<'a>(
    kind: &str,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    tag(kind.to_string(), TAG_LINE, palette)
}

/// One group: its label on the left, its places on the right.
fn group_row<'a>(
    group: PermissionGroupViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let label = text(group.label)
        .size(TEXT_SIZE)
        .font(fonts::UI)
        .color(palette.text_muted())
        .width(LABEL_WIDTH);

    let mut places = Column::new().spacing(PLACE_GAP).width(Length::Fill);
    for place in group.places {
        places = places.push(place_line(place, palette));
    }

    row![label, places].spacing(spacing::SPACE_4).into()
}

/// One place: its name in the monospace font (or its words), muted words
/// after it, and in the update dialog "new" or "no longer".
fn place_line<'a>(
    place: PlaceViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let color = match place.mark {
        Some(PlaceMark::New) => palette.status_warn(),
        Some(PlaceMark::NoLonger) => palette.text_muted(),
        None => palette.text_body(),
    };
    let font = if place.is_words {
        fonts::UI
    } else {
        fonts::MONO
    };

    let mut line = Row::new()
        .spacing(spacing::SPACE_2)
        .align_y(Alignment::Center)
        .push(
            text(place.text)
                .size(TEXT_SIZE)
                .font(font)
                .color(color)
                .wrapping(Wrapping::WordOrGlyph),
        );
    if let Some(note) = place.note {
        line = line.push(
            text(note)
                .size(TEXT_SIZE)
                .font(fonts::UI)
                .color(palette.text_muted()),
        );
    }
    match place.mark {
        Some(PlaceMark::New) => {
            line = line.push(
                text(i18n::t(Key::PluginsMarkNew))
                    .size(typography::CAPTION.size())
                    .font(fonts::UI_SEMIBOLD)
                    .color(palette.status_warn()),
            );
        },
        Some(PlaceMark::NoLonger) => {
            line = line.push(
                text(i18n::t(Key::PluginsMarkNoLonger))
                    .size(typography::CAPTION.size())
                    .font(fonts::UI)
                    .color(palette.text_muted()),
            );
        },
        None => {},
    }

    line.wrap().vertical_spacing(2.0).into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::model::permissions::PermissionGroup;

    fn place(text: &str, mark: Option<PlaceMark>) -> PlaceViewModel {
        PlaceViewModel {
            group: PermissionGroup::Files,
            text: String::from(text),
            is_words: false,
            note: None,
            mark,
        }
    }

    /// Issue 35: the groups show under "Can access" with its muted line,
    /// each label beside its places, the update marks after a place.
    #[test]
    fn the_groups_show_under_can_access() {
        let palette = UiPalette::otty_dark();
        let groups = vec![PermissionGroupViewModel {
            label: i18n::t(Key::PluginsGroupFiles),
            places: vec![
                place("~/.x/**", None),
                place("~/.y/**", Some(PlaceMark::New)),
                place("~/.z/**", Some(PlaceMark::NoLonger)),
            ],
        }];
        let mut ui = simulator(can_access(groups, &palette));

        let label = ui
            .find(i18n::t(Key::PluginsGroupFiles))
            .expect("the label")
            .bounds();
        let first = ui.find("~/.x/**").expect("a place").bounds();
        let second = ui.find("~/.y/**").expect("a place").bounds();

        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccessNote)).is_ok());
        assert!(ui.find("~/.z/**").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsMarkNew)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsMarkNoLonger)).is_ok());
        assert!(label.x < first.x);
        assert!((label.y - first.y).abs() < 1.0, "{label:?} {first:?}");
        assert!(first.y < second.y);
    }

    #[test]
    fn a_part_with_no_permission_says_so() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(can_access(Vec::new(), &palette));

        assert!(ui.find(i18n::t(Key::PluginsNoPermissions)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_err());
    }
}
