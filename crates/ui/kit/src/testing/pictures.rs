//! Picture checks: that a view draws a saved picture, and that an icon
//! lights up over its whole control.

use std::path::Path;

use iced::widget::container;
use iced::{Element, Event, Point, Rectangle, Size, Vector, mouse};

use super::temp_dir::TempDir;

/// Where the tested control is placed in the test window.
const ORIGIN: f32 = 40.0;
/// A window for one small control and its tooltip, as [`draws`] places
/// them.
pub(crate) const SMALL_WINDOW: Size = Size::new(240.0, 160.0);

/// Draws `view` with no pointer, with the pointer on the middle of
/// `control` (a rectangle inside `view`) and with the pointer one pixel
/// inside its top-left corner, away from the icon. The corner picture
/// must equal the middle one (the icon lights up over the whole control)
/// and both must differ from the idle one.
pub(crate) fn assert_icon_lights_up(
    name: &str,
    view: impl Fn() -> Element<'static, ()>,
    control: Rectangle,
) {
    let folder = TempDir::new("icon-hover");
    let reference = folder.path().join(name);
    let middle = control.center();
    let corner = Point::new(control.x + 1.0, control.y + 1.0);

    // Saves the picture of the pointer on the icon.
    assert!(
        draws(&view, SMALL_WINDOW, Some(middle), &reference),
        "{name}: save"
    );

    assert!(
        draws(&view, SMALL_WINDOW, Some(corner), &reference),
        "{name}: the icon must light up with the pointer on the corner"
    );
    assert!(
        !draws(&view, SMALL_WINDOW, None, &reference),
        "{name}: the idle picture must differ from the hovered one"
    );
}

/// Says whether `view`, drawn in a `window` with the pointer at `pointer`
/// (or with no pointer), gives the picture saved at `reference`; the first
/// call saves it. The view stands 40px in from the top left corner of the
/// window, and `pointer` is a point of the view, from its top left corner.
pub(crate) fn draws(
    view: &impl Fn() -> Element<'static, ()>,
    window: Size,
    pointer: Option<Point>,
    reference: &Path,
) -> bool {
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        window,
        container(view()).padding(ORIGIN),
    );
    if let Some(point) = pointer {
        let position = point + Vector::new(ORIGIN, ORIGIN);
        ui.point_at(position);
        let _ =
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    }

    ui.snapshot(&iced::Theme::Dark)
        .expect("view should draw")
        .matches_hash(reference)
        .expect("reference should be readable")
}
