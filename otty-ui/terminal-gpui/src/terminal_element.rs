use gpui::{
    App, Bounds, ContentMask, Element, ElementId, ElementInputHandler, Entity,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, PaintQuad,
    Pixels, ShapedLine, StrikethroughStyle, Style, TextRun, UnderlineStyle,
    Window, fill, point, px, relative, size,
};
use otty_libterm::escape::{CursorShape, StdColor};
use otty_libterm::surface::{Flags, Point, SnapshotView};

use crate::render_runs::build_render_runs;
use crate::{CellMetrics, Terminal, TerminalConfig};

pub(crate) struct TerminalElement {
    terminal: Entity<Terminal>,
}

impl TerminalElement {
    pub(crate) fn new(terminal: Entity<Terminal>) -> Self {
        Self { terminal }
    }
}

struct PositionedLine {
    origin: gpui::Point<Pixels>,
    line: ShapedLine,
}

impl PositionedLine {
    fn new(origin: gpui::Point<Pixels>, line: ShapedLine) -> Self {
        Self { origin, line }
    }
}

#[derive(Default)]
pub(crate) struct TerminalPrepaintState {
    backgrounds: Vec<PaintQuad>,
    block_highlights: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    lines: Vec<PositionedLine>,
    line_height: Pixels,
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = TerminalPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some("otty-terminal-grid".into())
    }

    fn source_location(
        &self,
    ) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let (config, snapshot, marked_text, selected_block, is_focused) = {
            let terminal = self.terminal.read(cx);
            (
                terminal.config().clone(),
                terminal.snapshot_arc(),
                terminal.marked_text().to_string(),
                terminal.selected_block().cloned(),
                terminal.focus_handle_for_element().is_focused(window),
            )
        };
        let Some(metrics) = measure_cell(&config, window) else {
            return TerminalPrepaintState::default();
        };

        self.terminal.update(cx, |terminal, cx| {
            terminal.update_layout(bounds, metrics, cx);
        });

        let view = snapshot.view();
        let mut state = TerminalPrepaintState {
            backgrounds: cell_backgrounds(&view, &config, bounds, metrics),
            block_highlights: block_highlights(
                &view,
                &config,
                selected_block.as_ref(),
                bounds,
                metrics,
            ),
            cursor: cursor_bounds(
                view.cursor.shape,
                view.cursor.point,
                view.display_offset,
                bounds,
                metrics,
            )
            .map(|cursor| {
                fill(
                    cursor,
                    config
                        .theme()
                        .palette()
                        .resolve(
                            otty_libterm::escape::Color::Std(StdColor::Cursor),
                            view.colors,
                        )
                        .hsla(),
                )
            }),
            lines: shape_terminal_lines(
                &view, &config, bounds, metrics, is_focused, window,
            ),
            line_height: px(metrics.height()),
        };
        if !marked_text.is_empty() {
            state.lines.push(shape_marked_text(
                &marked_text,
                &view,
                &config,
                bounds,
                metrics,
                window,
            ));
        }

        state
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.terminal.read(cx).focus_handle_for_element();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.terminal.clone()),
            cx,
        );

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for background in prepaint.backgrounds.drain(..) {
                window.paint_quad(background);
            }
            for highlight in prepaint.block_highlights.drain(..) {
                window.paint_quad(highlight);
            }
            if focus_handle.is_focused(window)
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
            for positioned in prepaint.lines.drain(..) {
                let _ = positioned.line.paint(
                    positioned.origin,
                    prepaint.line_height,
                    window,
                    cx,
                );
            }
        });
    }
}

fn measure_cell(
    config: &TerminalConfig,
    window: &Window,
) -> Option<CellMetrics> {
    let font_size = px(config.font().size());
    let run = TextRun {
        len: 1,
        font: config.font().gpui_font(),
        color: config.theme().palette().foreground().hsla(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped =
        window
            .text_system()
            .shape_line("m".into(), font_size, &[run], None);
    let measured_width = f32::from(shaped.width);
    let width = if measured_width.is_finite() && measured_width > 0.0 {
        measured_width
    } else {
        config.font().size() * 0.6
    };
    let height = config.font().size() * config.font().line_height();
    let metrics = CellMetrics::try_new(width, height).ok()?;

    Some(metrics.snapped(window.scale_factor()))
}

fn cell_backgrounds(
    view: &SnapshotView<'_>,
    config: &TerminalConfig,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
) -> Vec<PaintQuad> {
    let mut backgrounds = Vec::new();
    let default = config.theme().palette().resolve(
        otty_libterm::escape::Color::Std(StdColor::Background),
        view.colors,
    );

    for indexed in view.cells {
        if indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }

        let selected = view
            .selection
            .is_some_and(|selection| selection.contains(indexed.point));
        let mut foreground = config
            .theme()
            .palette()
            .resolve(indexed.cell.fg, view.colors);
        let mut background = config
            .theme()
            .palette()
            .resolve(indexed.cell.bg, view.colors);
        if indexed.cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut foreground, &mut background);
        }
        if selected {
            background = config.theme().selection_background();
        }
        if background == default && !selected {
            continue;
        }

        let width = if indexed.cell.flags.contains(Flags::WIDE_CHAR) {
            metrics.width() * 2.0
        } else {
            metrics.width()
        };
        let origin =
            grid_origin(indexed.point, view.display_offset, bounds, metrics);
        backgrounds.push(fill(
            Bounds::new(origin, size(px(width), px(metrics.height()))),
            background.hsla(),
        ));
    }

    backgrounds
}

fn block_highlights(
    view: &SnapshotView<'_>,
    config: &TerminalConfig,
    selected_block: Option<&crate::BlockId>,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
) -> Vec<PaintQuad> {
    if view
        .mode
        .contains(otty_libterm::surface::SurfaceMode::ALT_SCREEN)
        || config.behavior().block_ui_mode() != crate::BlockUiMode::Internal
    {
        return Vec::new();
    }
    let Some(selected_block) = selected_block else {
        return Vec::new();
    };
    let Some(block) = view
        .blocks()
        .iter()
        .find(|block| block.meta.id == selected_block.as_str())
    else {
        return Vec::new();
    };
    let mut color = config.theme().block_highlight().hsla();
    color.a = color.a.min(0.1);
    let y = bounds.top()
        + px((block.start_line as f32 + view.display_offset as f32)
            * metrics.height());
    let height = block.line_count as f32 * metrics.height();

    vec![fill(
        Bounds::new(
            point(bounds.left(), y),
            size(bounds.size.width, px(height)),
        ),
        color,
    )]
}

fn shape_terminal_lines(
    view: &SnapshotView<'_>,
    config: &TerminalConfig,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
    cursor_is_focused: bool,
    window: &Window,
) -> Vec<PositionedLine> {
    build_render_runs(
        view,
        cursor_is_focused,
        config.theme(),
        config.font().gpui_font(),
    )
    .into_iter()
    .map(|run| {
        let text_runs = run
            .spans()
            .iter()
            .map(|span| TextRun {
                len: span.byte_range().len(),
                font: run.font().clone(),
                color: span.foreground(),
                background_color: None,
                underline: span.is_underlined().then_some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(span.foreground()),
                    wavy: false,
                }),
                strikethrough: span.is_struck_through().then_some(
                    StrikethroughStyle {
                        thickness: px(1.0),
                        color: Some(span.foreground()),
                    },
                ),
            })
            .collect::<Vec<_>>();
        let glyph_count = run.text().chars().count();
        let force_width =
            forced_cell_width(glyph_count, run.cell_columns(), metrics.width());
        let line = window.text_system().shape_line(
            run.text().to_string().into(),
            px(config.font().size()),
            &text_runs,
            force_width,
        );
        let origin = point(
            bounds.left() + px(run.start_column() as f32 * metrics.width()),
            bounds.top()
                + px((run.line() as f32 + view.display_offset as f32)
                    * metrics.height()),
        );

        PositionedLine::new(origin, line)
    })
    .collect()
}

fn shape_marked_text(
    text: &str,
    view: &SnapshotView<'_>,
    config: &TerminalConfig,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
    window: &Window,
) -> PositionedLine {
    let color = config.theme().selection_foreground().hsla();
    let text_run = TextRun {
        len: text.len(),
        font: config.font().gpui_font(),
        color,
        background_color: Some(config.theme().selection_background().hsla()),
        underline: Some(UnderlineStyle {
            thickness: px(1.0),
            color: Some(color),
            wavy: false,
        }),
        strikethrough: None,
    };
    let line = window.text_system().shape_line(
        text.to_string().into(),
        px(config.font().size()),
        &[text_run],
        Some(px(metrics.width())),
    );
    let origin =
        grid_origin(view.cursor.point, view.display_offset, bounds, metrics);

    PositionedLine::new(origin, line)
}

fn cursor_bounds(
    shape: CursorShape,
    terminal_point: Point,
    display_offset: usize,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
) -> Option<Bounds<Pixels>> {
    let origin = grid_origin(terminal_point, display_offset, bounds, metrics);
    let thickness = 2.0_f32.min(metrics.width()).min(metrics.height());

    match shape {
        CursorShape::Block => Some(Bounds::new(
            origin,
            size(px(metrics.width()), px(metrics.height())),
        )),
        CursorShape::Beam => Some(Bounds::new(
            origin,
            size(px(thickness), px(metrics.height())),
        )),
        CursorShape::Underline => Some(Bounds::new(
            point(origin.x, origin.y + px(metrics.height() - thickness)),
            size(px(metrics.width()), px(thickness)),
        )),
        CursorShape::Hidden => None,
    }
}

fn grid_origin(
    terminal_point: Point,
    display_offset: usize,
    bounds: Bounds<Pixels>,
    metrics: CellMetrics,
) -> gpui::Point<Pixels> {
    point(
        bounds.left() + px(terminal_point.column.0 as f32 * metrics.width()),
        bounds.top()
            + px((terminal_point.line.0 as f32 + display_offset as f32)
                * metrics.height()),
    )
}

fn forced_cell_width(
    character_count: usize,
    cell_columns: usize,
    cell_width: f32,
) -> Option<Pixels> {
    (character_count == cell_columns).then_some(px(cell_width))
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};
    use otty_libterm::escape::CursorShape;
    use otty_libterm::surface::{Column, Line, Point};

    use super::*;
    use crate::CellMetrics;

    #[test]
    fn cursor_shapes_stay_inside_the_target_cell() {
        let bounds =
            Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(60.0)));
        let metrics = CellMetrics::try_new(8.0, 16.0).expect("valid metrics");
        let point = Point::new(Line(1), Column(2));

        let block =
            cursor_bounds(CursorShape::Block, point, 0, bounds, metrics)
                .expect("visible block cursor");
        let bar = cursor_bounds(CursorShape::Beam, point, 0, bounds, metrics)
            .expect("visible bar cursor");
        let underline =
            cursor_bounds(CursorShape::Underline, point, 0, bounds, metrics)
                .expect("visible underline cursor");

        assert_eq!(block.origin, gpui::point(px(26.0), px(36.0)));
        assert_eq!(block.size, gpui::size(px(8.0), px(16.0)));
        assert_eq!(bar.size.width, px(2.0));
        assert_eq!(underline.size.height, px(2.0));
        assert!(
            cursor_bounds(CursorShape::Hidden, point, 0, bounds, metrics)
                .is_none()
        );
    }

    #[test]
    fn fixed_glyph_spacing_is_disabled_for_complex_cell_clusters() {
        assert_eq!(forced_cell_width(3, 3, 8.0), Some(px(8.0)));
        assert_eq!(forced_cell_width(2, 1, 8.0), None);
        assert_eq!(forced_cell_width(1, 2, 8.0), None);
    }
}
