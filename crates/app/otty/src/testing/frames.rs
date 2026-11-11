//! The whole window in a small size, frame after frame, as the running app
//! builds it: how far a screen or a dialog is scrolled, and the scrollers
//! and panel splits the scrolling tests press and drag.

use iced::Point;
use otty_ui_kit::size;

use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::message::AppEvent;
use crate::view::view;

/// A window where the main area between the panels at their smallest is
/// too narrow and too low for the start card. Panels dragged wide do the
/// same in a larger window.
pub(crate) const SMALL_WINDOW: iced::Size = iced::Size::new(400.0, 250.0);

/// One frame of the app after another, as the running app builds them: the
/// widget state carries over from one to the next.
pub(crate) struct Frames {
    renderer: iced::Renderer,
    cache: Option<iced_test::runtime::user_interface::Cache>,
}

impl Frames {
    /// Start with no frame built.
    pub(crate) fn new() -> Self {
        Self {
            renderer: headless_renderer(),
            cache: Some(iced_test::runtime::user_interface::Cache::default()),
        }
    }

    /// Build a frame of `app`, give it the wheel at the pointer if any, and
    /// return how far the scroller of the screen is scrolled.
    pub(crate) fn frame(
        &mut self,
        app: &App,
        wheel: Option<(Point, iced::Vector)>,
    ) -> iced::Vector {
        self.scrolled_frame(app, wheel, |content| {
            content.width == size::SCREEN_MIN
        })
        .expect("the screen should have its scroller")
    }

    /// Like [`Frames::frame`], for the scroller of the Settings dialog: the
    /// only one whose content is wider than the dialog's card.
    pub(crate) fn dialog_frame(
        &mut self,
        app: &App,
        wheel: Option<(Point, iced::Vector)>,
    ) -> iced::Vector {
        self.scrolled_frame(app, wheel, |content| {
            content.width > size::DIALOG_WIDE
        })
        .expect("the dialog should have its scroller")
    }

    /// Build a frame of `app`, give it the wheel at the pointer if any, and
    /// return how far the scroller whose content `is_scroller` picks is
    /// scrolled.
    fn scrolled_frame(
        &mut self,
        app: &App,
        wheel: Option<(Point, iced::Vector)>,
        is_scroller: fn(&iced::Rectangle) -> bool,
    ) -> Option<iced::Vector> {
        use iced::advanced::widget::{Operation, operation};
        use iced_test::runtime::user_interface::UserInterface;
        use iced_test::selector::{Candidate, Selector};

        let mut ui = UserInterface::build(
            view(app),
            SMALL_WINDOW,
            self.cache.take().expect("the frames keep their cache"),
            &mut self.renderer,
        );
        if let Some((pointer, delta)) = wheel {
            let mut messages = Vec::new();
            let _ = ui.update(
                &[iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Pixels {
                        x: delta.x,
                        y: delta.y,
                    },
                })],
                iced::mouse::Cursor::Available(pointer),
                &mut self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
        }

        let mut find = (move |candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                content_bounds,
                translation,
                ..
            } if is_scroller(&content_bounds) => Some(translation),
            _ => None,
        })
        .find();
        ui.operate(&self.renderer, &mut operation::black_box(&mut find));
        let offset = match find.finish() {
            operation::Outcome::Some(Some(offset)) => Some(offset),
            _ => None,
        };

        self.cache = Some(ui.into_cache());

        offset
    }
}

/// A renderer that draws nothing on screen, for building the window in
/// tests.
pub(crate) fn headless_renderer() -> iced::Renderer {
    use iced_test::core::renderer::Headless;

    iced_test::runtime::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            None,
        ),
    )
    .expect("a headless renderer")
}

/// A wheel in the main area of a [`SMALL_WINDOW`] that moves a page 30 px
/// right and 50 px down.
pub(crate) fn scroll_main_area() -> Option<(Point, iced::Vector)> {
    Some((Point::new(200.0, 150.0), iced::Vector::new(-30.0, -50.0)))
}

/// The scroller of the main area: where it is and how far it is scrolled.
/// It is the only one whose content has the minimum width of a screen, and
/// the tests that use this one need a page that overflows it both ways.
pub(crate) fn card_area_scroller(
    ui: &mut iced_test::Simulator<'_, AppEvent>,
) -> (iced::Rectangle, iced::Vector) {
    use iced_test::selector::Candidate;

    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Scrollable {
            bounds,
            content_bounds,
            translation,
            ..
        } if content_bounds.width == size::SCREEN_MIN
            && content_bounds.height > bounds.height =>
        {
            Some((bounds, translation))
        },
        _ => None,
    })
    .expect("the card should overflow the main area both ways")
}

/// How many scrollers in `app` have content of the minimum width of a
/// screen.
pub(crate) fn screen_scrollers(app: &App) -> usize {
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        SMALL_WINDOW,
        view(app),
    );
    let mut found = 0;

    let _ = ui.find(|candidate: iced_test::selector::Candidate<'_>| {
        if let iced_test::selector::Candidate::Scrollable {
            content_bounds, ..
        } = candidate
            && content_bounds.width == size::SCREEN_MIN
        {
            found += 1;
        }
        None::<()>
    });

    found
}

/// Press at the first point, move along the others and stay pressed.
pub(crate) fn press_and_move(
    ui: &mut iced_test::Simulator<'_, AppEvent>,
    points: &[Point],
) {
    for (index, position) in points.iter().enumerate() {
        ui.point_at(*position);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved {
                position: *position,
            },
        )]);
        if index == 0 {
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left),
            )]);
        }
    }
}

/// Whether the window sent a resize of a panel.
pub(crate) fn sent_a_panel_resize(
    ui: iced_test::Simulator<'_, AppEvent>,
) -> bool {
    ui.into_messages().any(|event| {
        matches!(event, AppEvent::Sidebar(SidebarIntent::Resized(_)))
    })
}

/// The x of the panel split at the right of the main area.
pub(crate) fn right_panel_split(app: &App, window: iced::Size) -> f32 {
    panel_split_lines(app, window).fold(0.0, f32::max)
}

/// The x of the panel split at the left of the main area.
pub(crate) fn left_panel_split(app: &App, window: iced::Size) -> f32 {
    panel_split_lines(app, window).fold(f32::MAX, f32::min)
}

/// The x of each line between the panels and the main area.
fn panel_split_lines(
    app: &App,
    window: iced::Size,
) -> impl Iterator<Item = f32> {
    app.sidebar
        .panes()
        .layout()
        .split_regions(size::LINE, 0.0, window)
        .into_values()
        .map(|(axis, region, ratio)| {
            axis.split_line_bounds(region, ratio, size::LINE).x
        })
}
