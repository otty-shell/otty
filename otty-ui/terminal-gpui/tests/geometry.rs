use otty_libterm::surface::{Column, Line};
use otty_ui_term_gpui::{CellMetrics, TerminalGeometry};

#[test]
fn derives_grid_size_from_inner_widget_bounds() {
    let geometry = TerminalGeometry::new(800.0, 480.0, 9.0, 1.0);
    let metrics = CellMetrics::try_new(10.0, 20.0).expect("valid metrics");

    let size = geometry.terminal_size(metrics);

    assert_eq!(size.cols, 78);
    assert_eq!(size.rows, 23);
    assert_eq!(size.cell_width, 10);
    assert_eq!(size.cell_height, 20);
}

#[test]
fn clamps_small_widget_and_pointer_to_a_safe_grid() {
    let geometry = TerminalGeometry::new(2.0, 2.0, 9.0, 1.0);
    let metrics = CellMetrics::try_new(8.0, 16.0).expect("valid metrics");
    let size = geometry.terminal_size(metrics);

    assert_eq!((size.cols, size.rows), (1, 1));
    let point = geometry.point_to_grid(10_000.0, 10_000.0, metrics, 4);
    assert_eq!(point.column, Column(0));
    assert_eq!(point.line, Line(-4));
}

#[test]
fn snaps_metrics_to_device_pixels() {
    let metrics = CellMetrics::try_new(7.3, 15.1).expect("valid metrics");

    let snapped = metrics.snapped(1.25);

    assert_eq!(snapped.width(), 7.2);
    assert_eq!(snapped.height(), 15.2);
}

#[test]
fn snapping_keeps_tiny_valid_metrics_positive() {
    let metrics = CellMetrics::try_new(0.1, 0.2).expect("valid metrics");

    let snapped = metrics.snapped(1.0);

    assert!(snapped.width() > 0.0);
    assert!(snapped.height() > 0.0);
}
