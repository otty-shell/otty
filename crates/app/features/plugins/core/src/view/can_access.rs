//! What a part can access (issues 35, 36): the dialog "<Part> can access"
//! that Can access on the card of a part opens, with the muted line that
//! everything else is refused over a card of the groups alone, as its title
//! names the part; and the card of a part in the install and update
//! dialogs: its name, the tag of its kind and its description, a thin rule,
//! "Can access" and its groups. Both show the groups alike: one row per
//! group with a thin rule between rows, its label with one muted line of
//! what it means in a column on the left, its places on the right one per
//! line in the monospace font.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, container, row, text};
use iced::{Alignment, Border, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tag::tag;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use crate::event::PluginsIntent;
use crate::model::permissions::{
    PermissionGroupViewModel, PlaceMark, PlaceViewModel,
};
use crate::model::{CanAccessDialogViewModel, PartPermissionsViewModel};

/// Width of the label column of a part card, room for "Hidden terminal"
/// and the lines of what the groups mean.
const LABEL_WIDTH: f32 = 180.0;
/// Text size of the groups, as the facts of the page (`font-size:
/// 12.5px`).
const TEXT_SIZE: f32 = 12.5;
/// Room above and below the rule between two group rows (`padding: 10px
/// 0`).
const ROW_PADDING: f32 = 10.0;
/// Gap between the places of a group.
const PLACE_GAP: f32 = 3.0;
/// Gap between a label and the line of what it means.
const MEANING_GAP: f32 = 2.0;
/// Height of the line the kind tag is centred in: the tag is 16 px high.
const TAG_LINE: f32 = 20.0;

/// The dialog "<Part> can access": the muted line that OTTY refuses
/// everything else, a card of the groups alone, and Close.
pub(super) fn dialog<'a>(
    dialog: CanAccessDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let body = column![
        small_text(i18n::t(Key::PluginsCanAccessNote), palette.text_muted()),
        card(permission_groups(dialog.groups, palette), palette),
    ]
    .spacing(spacing::SPACE_4);

    let close = Button::new(i18n::t(Key::ButtonClose), palette)
        .kind(ButtonKind::Secondary)
        .on_press(PluginsIntent::Back);

    Dialog::new(dialog.title, palette)
        .width(size::DIALOG_WIDE)
        .on_close(PluginsIntent::Back, i18n::t(Key::ButtonClose))
        .body(body)
        .action(close)
        .into()
}

/// A part in the install or update dialog as a compact card: its name,
/// the tag of its kind, its mark in the update dialog and its
/// description; then, unless the new version no longer has it, a thin
/// rule, "Can access" when the part asks for something, and its groups.
pub(super) fn part_card<'a>(
    part: PartPermissionsViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut title = row![
        text(part.name)
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        kind_tag(part.kind, palette),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);
    if let Some(mark) = part.mark {
        title = title.push(small_text(mark, palette.status_warn()));
    }
    let mut head = column![title].spacing(spacing::SPACE_1);
    if let Some(description) = part.description {
        head = head.push(small_text(description, palette.text_muted()));
    }

    let mut content = column![head].spacing(spacing::SPACE_4);
    if !part.is_removed {
        let mut access =
            column![divider::horizontal(palette)].spacing(spacing::SPACE_3);
        if !part.groups.is_empty() {
            access = access.push(
                text(i18n::t(Key::PluginsCanAccess))
                    .size(TEXT_SIZE)
                    .font(fonts::UI_SEMIBOLD)
                    .color(palette.text_strong()),
            );
        }
        access = access.push(permission_groups(part.groups, palette));
        content = content.push(access);
    }

    card(content, palette)
}

/// The kind of a part, such as "Agent", as a small tag centred on the line
/// of the part's name.
pub(super) fn kind_tag<'a>(
    kind: &str,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    tag(kind.to_string(), TAG_LINE, palette)
}

/// `content` on a compact card of the panel surface with a thin border.
fn card<'a>(
    content: impl Into<Element<'a, PluginsIntent>>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let background = palette.surface_panel();
    let line = palette.line();

    container(content)
        .width(Length::Fill)
        .padding(spacing::SPACE_4)
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_MD.into(),
                width: size::LINE,
                color: line,
            },
            ..container::Style::default()
        })
        .into()
}

/// The groups of a part, each label with what it means under it, or "Asks
/// for no permissions" when there are none.
fn permission_groups<'a>(
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

    // A thin rule between the rows, so a label, what it means and its
    // places read as one block.
    let mut rows = Column::new().spacing(ROW_PADDING);
    for (index, group) in groups.into_iter().enumerate() {
        if index > 0 {
            rows = rows.push(divider::horizontal(palette));
        }
        rows = rows.push(group_row(group, palette));
    }

    rows.into()
}

/// One group: its label and what it means on the left, its places on the
/// right.
fn group_row<'a>(
    group: PermissionGroupViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut label = column![
        text(group.label)
            .size(TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_body())
    ]
    .spacing(MEANING_GAP)
    .width(LABEL_WIDTH);
    if let Some(meaning) = group.meaning {
        label = label.push(
            text(meaning)
                .size(typography::CAPTION.size())
                .font(fonts::UI)
                .color(palette.text_muted()),
        );
    }

    let mut places = Column::new().spacing(PLACE_GAP).width(Length::Fill);
    for place in group.places {
        places = places.push(place_line(place, palette));
    }

    row![label, places].spacing(spacing::SPACE_4).into()
}

/// One place: muted words before it, its name in the monospace font (or
/// its words), muted words after it, and in the update dialog "new" or
/// "no longer".
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
    let muted = |words: &'static str| {
        text(words)
            .size(TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted())
    };

    let mut line = Row::new()
        .spacing(spacing::SPACE_2)
        .align_y(Alignment::Center);
    if let Some(lead) = place.lead {
        line = line.push(muted(lead));
    }
    line = line.push(
        text(place.text)
            .size(TEXT_SIZE)
            .font(font)
            .color(color)
            .wrapping(Wrapping::WordOrGlyph),
    );
    if let Some(note) = place.note {
        line = line.push(muted(note));
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
        _ => {},
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
            lead: None,
            note: None,
            mark,
        }
    }

    fn part(groups: Vec<PermissionGroupViewModel>) -> PartPermissionsViewModel {
        PartPermissionsViewModel {
            name: String::from("Example Cloud"),
            kind: "Provider",
            description: Some(String::from(
                "Reads the limits of Example Cloud accounts.",
            )),
            mark: None,
            is_removed: false,
            groups,
        }
    }

    fn found_logins() -> PermissionGroupViewModel {
        PermissionGroupViewModel {
            label: i18n::t(Key::PluginsGroupFoundLogins),
            meaning: Some(String::from(
                "Logins and keys of Example Cloud accounts",
            )),
            places: vec![
                PlaceViewModel {
                    group: PermissionGroup::FoundLogins,
                    is_words: true,
                    ..place("Example Cloud accounts", None)
                },
                PlaceViewModel {
                    group: PermissionGroup::FoundLogins,
                    lead: Some(i18n::t(Key::PluginsPlaceKeysFor)),
                    ..place("example.com", None)
                },
            ],
        }
    }

    /// Issue 36: the dialog of a part says that only this is open to it,
    /// then shows its groups alone, no name, kind or description, as its
    /// title names the part: each group with what it means and its
    /// places, the keys of a host after "keys for". Close closes it.
    #[test]
    fn the_can_access_dialog_shows_the_line_and_the_groups() {
        let palette = UiPalette::otty_dark();
        let vm = CanAccessDialogViewModel {
            title: String::from("Example Cloud can access"),
            groups: vec![found_logins()],
        };
        let mut ui = simulator(dialog(vm, &palette));

        let note = ui
            .find(i18n::t(Key::PluginsCanAccessNote))
            .expect("the line")
            .bounds();
        let label = ui
            .find(i18n::t(Key::PluginsGroupFoundLogins))
            .expect("the label")
            .bounds();
        let meaning = ui
            .find("Logins and keys of Example Cloud accounts")
            .expect("what it means")
            .bounds();
        let lead = ui
            .find(i18n::t(Key::PluginsPlaceKeysFor))
            .expect("the lead")
            .bounds();
        let host = ui.find("example.com").expect("the host").bounds();

        assert!(ui.find("Example Cloud can access").is_ok());
        assert!(ui.find("Example Cloud").is_err());
        assert!(ui.find("Provider").is_err());
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_err());
        assert!(ui.find("Example Cloud accounts").is_ok());
        assert!(note.y < label.y);
        assert!(meaning.y > label.y && (meaning.x - label.x).abs() < 1.0);
        assert!(label.x < lead.x && lead.x < host.x);
        let _ = ui.click(i18n::t(Key::ButtonClose));
        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(sent.as_slice(), [PluginsIntent::Back]));
    }

    /// Issue 35, 36: in the install and update dialogs a part shows its
    /// kind and description, "Can access" titles the groups, each label
    /// beside its places, the update marks after a place.
    #[test]
    fn the_card_of_a_part_marks_what_an_update_changes() {
        let palette = UiPalette::otty_dark();
        let groups = vec![PermissionGroupViewModel {
            label: i18n::t(Key::PluginsGroupFiles),
            meaning: None,
            places: vec![
                place("~/.x/**", None),
                place("~/.y/**", Some(PlaceMark::New)),
                place("~/.z/**", Some(PlaceMark::NoLonger)),
            ],
        }];
        let mut ui = simulator(part_card(part(groups), &palette));

        let title = ui
            .find(i18n::t(Key::PluginsCanAccess))
            .expect("the title")
            .bounds();
        let label = ui
            .find(i18n::t(Key::PluginsGroupFiles))
            .expect("the label")
            .bounds();
        let first = ui.find("~/.x/**").expect("a place").bounds();
        let second = ui.find("~/.y/**").expect("a place").bounds();

        assert!(ui.find("Provider").is_ok());
        assert!(
            ui.find("Reads the limits of Example Cloud accounts.")
                .is_ok()
        );
        assert!(ui.find("~/.z/**").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsMarkNew)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsMarkNoLonger)).is_ok());
        assert!(title.y < label.y);
        assert!(label.x < first.x);
        assert!((label.y - first.y).abs() < 1.0, "{label:?} {first:?}");
        assert!(first.y < second.y);
    }

    /// A part that asks for nothing says so, with no "Can access".
    #[test]
    fn a_part_with_no_permission_says_so() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(part_card(part(Vec::new()), &palette));

        assert!(ui.find(i18n::t(Key::PluginsNoPermissions)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_err());
    }

    /// A part the new version no longer has shows its name and mark only.
    #[test]
    fn a_removed_part_shows_no_groups() {
        let palette = UiPalette::otty_dark();
        let removed = PartPermissionsViewModel {
            mark: Some("no longer in the plugin"),
            is_removed: true,
            ..part(Vec::new())
        };
        let mut ui = simulator(part_card(removed, &palette));

        assert!(ui.find("no longer in the plugin").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsNoPermissions)).is_err());
    }
}
